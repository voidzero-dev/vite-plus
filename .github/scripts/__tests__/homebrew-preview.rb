# frozen_string_literal: true

# Run with `brew ruby` inside the disposable Homebrew e2e prefix.
require "formulary"

def check(condition, message)
  raise message unless condition
end

def rejects(message)
  yield
rescue StandardError => e
  raise unless e.message.include?(message)
else
  raise "Expected failure: #{message}"
end

source = Pathname.new(__dir__).join("../../../HomebrewFormula/vp.rb").realpath
load_formula = lambda do
  Formulary.clear_cache
  Formulary.from_contents("vp", source, source.read)
end

# Metadata requests are fixtures. Archive downloads use Homebrew's real cache
# and resource implementation, with bytes placed at the expected cache key.
metadata = nil
requests = []
commit = "a" * 40
commit_key = "voidzero-dev:vite-plus:#{commit}"
Utils::Curl.define_singleton_method(:curl_output) do |*args, **_options|
  requests << args.last
  raise "Unexpected metadata request" unless metadata

  Struct.new(:stdout) do
    def assert_success!; end
  end.new(JSON.generate(metadata))
end
Utils::Curl.define_singleton_method(:curl_headers) do |*args, **_options|
  requests << args.last
  { responses: [{ headers: { "x-commit-key" => commit_key } }] }
end

ENV.delete("HOMEBREW_VP_PR_VERSION")
stable = load_formula.call
check(stable.stable.url.start_with?("https://github.com/voidzero-dev/vite-plus/releases/"), "Stable URL changed")
check(requests.empty?, "Stable formula requested preview metadata")
preview = stable.class.const_get(:Preview)

seed = lambda do |platform, sha, bytes = "preview archive"|
  version = "0.0.0-commit.#{sha}"
  package = "@voidzero-dev/vite-plus-cli-#{platform}"
  shasum = Digest::SHA1.hexdigest(bytes)
  url = "https://registry-bridge.viteplus.dev/tarballs/#{package}/#{version}/#{shasum}.tgz"
  archive = HOMEBREW_CACHE/"downloads/#{Digest::SHA256.hexdigest(url)}--#{shasum}.tgz"
  archive.dirname.mkpath
  archive.write(bytes)
  cached_metadata = HOMEBREW_CACHE/"vp-preview/#{sha}-#{platform}.json"
  cached_metadata.unlink if cached_metadata.exist?
  metadata = {
    "name" => package, "version" => version,
    "dist" => { "tarball" => url, "shasum" => shasum,
                "integrity" => "sha512-#{Digest::SHA512.base64digest(bytes)}" },
  }
  archive
end

%w[../main main 0 12x abcdef0].each do |ref|
  rejects("PR number or a full commit SHA") { preview.resolve(ref, "darwin-arm64") }
end
check(requests.empty?, "Invalid selectors made network requests")

commit_key = ""
rejects("No published Vite+ preview") { preview.resolve("2740", "darwin-arm64") }
commit_key = "voidzero-dev:vite-plus:#{commit}"

%w[darwin-arm64 darwin-x64 linux-arm64-gnu linux-x64-gnu].each do |platform|
  archive = seed.call(platform, commit)
  result = preview.resolve("2740", platform)
  check(result[:version] == "0.0.0-commit.#{commit}", "PR did not resolve to a commit")
  check(result[:sha256] == Digest::SHA256.file(archive).hexdigest, "Wrong SHA-256")
  check(ENV["HOMEBREW_VP_PR_VERSION"] == commit, "Build subprocess was not pinned")
  count = requests.length
  check(preview.resolve(commit, platform) == result, "Cached preview changed")
  check(requests.length == count, "Pinned build requested metadata again")
end
puts "PR resolution, all four platforms, integrity verification, and offline build subprocesses pass."

platform = "#{OS.mac? ? "darwin" : "linux"}-#{Hardware::CPU.arm? ? "arm64" : "x64"}#{"-gnu" if OS.linux?}"
expected = preview.resolve(commit, platform)
ENV["HOMEBREW_VP_PR_VERSION"] = commit.upcase
selected = load_formula.call
check(selected.version.to_s == "0.0.0-commit.#{commit}", "Formula did not select the preview")
check(!selected.disabled?, "Preview kept the stable launch gate")
check(selected.stable.url == expected[:url], "Wrong formula URL")
check(selected.stable.checksum.to_s == Digest::SHA256.hexdigest("preview archive"), "Formula lost its checksum")

seed.call(platform, "b" * 40)
metadata["version"] = "0.3.3"
rejects("does not match") { preview.resolve("b" * 40, platform) }
seed.call(platform, "c" * 40)
metadata["dist"]["tarball"] = "https://example.test/untrusted.tgz"
rejects("invalid download URL or checksum") { preview.resolve("c" * 40, platform) }
seed.call(platform, "d" * 40)
metadata["dist"]["integrity"] = "sha512-invalid"
rejects("invalid download URL or checksum") { preview.resolve("d" * 40, platform) }
archive = seed.call(platform, "e" * 40)
archive.write("corrupted archive")
rejects("checksum mismatch") { preview.resolve("e" * 40, platform) }
check(!archive.exist?, "Corrupted download stayed in the cache")
puts "Wrong versions, foreign URLs, invalid integrity, and corrupt archives are rejected."

ENV.delete("HOMEBREW_VP_PR_VERSION")
restored = load_formula.call
check(restored.version == stable.version && restored.stable.url == stable.stable.url, "Stable selection did not return")
puts "Removing the selector restores the stable formula."
