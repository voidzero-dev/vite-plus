import argparse
import http.client
import json
import os
import re
import signal
import stat
import tempfile
import time
from pathlib import Path


MODEL = "@cf/deepseek-ai/deepseek-v4.1-flash-1017"
REQUEST_LIMIT = 256 * 1024


def http_error(response):
    message = f"Workers AI returned HTTP {response.status}"
    raw = response.read(65536).decode("utf-8", errors="replace")
    try:
        payload = json.loads(raw)
    except ValueError:
        return message
    if not isinstance(payload, dict):
        return message
    errors = payload.get("errors")
    error = errors[0] if isinstance(errors, list) and errors else payload.get("error", payload)
    if not isinstance(error, dict):
        return message
    code = str(error.get("code", ""))
    detail = str(error.get("message", "")).lower()
    if not re.fullmatch("[0-9]{1,7}", code) and re.search(r"\b5035\b", detail):
        code = "5035"
    if re.fullmatch("[0-9]{1,7}", code):
        message += f" (Cloudflare error {code})"
    if "free plan" in detail or "paid plan" in detail:
        message += ". The account plan restricts model access."
    return message


def stream_choices(stream):
    for event in stream.decode("utf-8").replace("\r\n", "\n").replace("\r", "\n").split("\n\n"):
        data = "\n".join(line[5:].lstrip(" ") for line in event.splitlines() if line.startswith("data:"))
        if not data or data == "[DONE]":
            continue
        chunk = json.loads(data)
        if "error" in chunk:
            raise SystemExit("Workers AI returned a streaming error")
        yield from chunk.get("choices", [])


def finish_result(stream):
    """Read one streamed function call without executing any tools."""
    name = None
    index = None
    arguments = []
    finished = False
    for choice in stream_choices(stream):
        delta = choice.get("delta") or {}
        if choice.get("index", 0) != 0 or "function_call" in delta:
            raise SystemExit("Unexpected completion choice")
        for call in delta.get("tool_calls", []):
            current = call["index"]
            if index is not None and current != index:
                raise SystemExit("Expected one tool call")
            index = current
            function = call.get("function", {})
            if function.get("name"):
                if function["name"] != "finish":
                    raise SystemExit("Expected the finish tool")
                name = function["name"]
            arguments.append(function.get("arguments") or "")
        if choice.get("finish_reason") in ("stop", "tool_calls"):
            finished = True
    if name != "finish" or not finished:
        raise SystemExit("Incomplete finish tool call")
    return json.loads("".join(arguments))


def expect_result(stage, result):
    if stage == "triage":
        if (result.get("reproducible") is False and result.get("skipped") is True
                and result.get("skippedReason") == "not-actionable"):
            return
    elif (result.get("status") == "inconclusive" and isinstance(result.get("reasoning"), str)
          and result["reasoning"].strip()):
        return
    raise SystemExit("Unexpected synthetic fixture result")


def fetch_stream(account, token, raw, label):
    conn = http.client.HTTPSConnection("api.cloudflare.com", timeout=150)
    try:
        conn.request("POST", f"/client/v4/accounts/{account}/ai/v1/chat/completions",
                     body=raw, headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json"})
        response = conn.getresponse()
        if response.status != 200:
            raise SystemExit(http_error(response))
        print(f"{label} received HTTP 200", flush=True)
        if response.headers.get_content_type() != "text/event-stream":
            raise SystemExit("Workers AI returned HTTP 200 without an event stream")
        return response.read()
    finally:
        conn.close()


def relay_loop(wire, account, token):
    if not wire.is_absolute():
        raise SystemExit("triage-loop requires an absolute wire directory")
    directory = os.open(wire, os.O_DIRECTORY | os.O_NOFOLLOW)
    # This wall-clock deadline also covers active HTTPS reads.
    signal.alarm(300)
    try:
        for number in range(1, 19):
            while True:
                try:
                    fd = os.open(f"request-{number}.json", os.O_RDONLY | os.O_NONBLOCK | os.O_NOFOLLOW,
                                 dir_fd=directory)
                    break
                except FileNotFoundError:
                    time.sleep(0.2)
            with os.fdopen(fd, "rb") as source:
                info = os.fstat(source.fileno())
                if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size > REQUEST_LIMIT:
                    raise SystemExit("Expected a regular request file of at most 256 KiB")
                raw = source.read(REQUEST_LIMIT + 1)
            if len(raw) > REQUEST_LIMIT:
                raise SystemExit("Request exceeds 256 KiB")
            body = json.loads(raw)
            if body.get("model") != MODEL or body.get("stream") is not True or not body.get("messages"):
                raise SystemExit("Unexpected streaming model request")
            finishes = [t["function"]["parameters"] for t in body["tools"]
                        if t.get("type") == "function" and t["function"].get("name") == "finish"]
            if len(finishes) != 1 or finishes[0].get("type") != "object" or set(
                finishes[0].get("required", [])
            ) not in ({"reproducible", "skipped", "skippedReason"}, {"result"}):
                raise SystemExit("Unexpected triage finish tool schema")

            response_name = f"response-{number}.sse"
            try:
                os.stat(response_name, dir_fd=directory, follow_symlinks=False)
            except FileNotFoundError:
                pass
            else:
                raise SystemExit("Response file already exists")
            label = f"triage-loop request {number}"
            stream = fetch_stream(account, token, raw, label)
            calls = {call["index"] for choice in stream_choices(stream)
                     for call in (choice.get("delta") or {}).get("tool_calls", [])}
            # Complete the regular file outside the shared directory, then publish
            # it atomically. Linking fails if a file or symlink already has this name.
            with tempfile.NamedTemporaryFile(dir=wire.parent) as output:
                output.write(stream)
                output.flush()
                os.fchmod(output.fileno(), 0o644)
                os.link(output.name, response_name, dst_dir_fd=directory)
            print(f"{label} published SSE with {len(calls)} tool calls", flush=True)
    finally:
        signal.alarm(0)
        os.close(directory)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("wire", type=Path)
    parser.add_argument("stage", choices=("triage", "verification", "triage-loop"))
    args = parser.parse_args()
    account = os.environ.get("CF_TEST_ACCOUNT", "")
    token = os.environ.get("CF_TEST_TOKEN", "")
    if not re.fullmatch("[0-9a-fA-F]{32}", account) or not token:
        raise SystemExit("Workers AI credentials are unavailable or invalid")

    if args.stage == "triage-loop":
        return relay_loop(args.wire, account, token)

    raw = (args.wire / "request-1.json").read_bytes()
    request = json.loads(raw)
    if request["model"] != MODEL or request["stream"] is not True:
        raise SystemExit("Expected the streaming DeepSeek request from the pinned action")

    stream = fetch_stream(account, token, raw, args.stage)
    expect_result(args.stage, finish_result(stream))
    with (args.wire / "response-1.sse").open("xb") as output:
        output.write(stream)
    print(f"{args.stage} returned the expected finish result")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, TypeError, KeyError, AttributeError, OSError, http.client.HTTPException) as error:
        raise SystemExit(f"Model smoke test failed ({type(error).__name__})") from None
