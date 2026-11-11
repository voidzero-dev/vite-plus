set -eu
vp_bin=$(command -v vp)
VP_COMPLETE=bash "$vp_bin" > completion.bash
source completion.bash
COMP_WORDBREAKS=' :='
COMP_TYPE=9
vp() {
    if [[ $_CLAP_COMPLETE_INDEX != "$expected_index" || $# -ne ${#expected_args[@]} ]]; then
        printf 'unexpected completion argv: index=%s count=%s expected_count=%s %s\n' "$_CLAP_COMPLETE_INDEX" "$#" "${#expected_args[@]}" "$*" >&2
        return 1
    fi
    local i
    local actual=("$@")
    for ((i = 0; i < $#; i++)); do
        if [[ "${actual[i]}" != "${expected_args[i]}" ]]; then
            printf 'arg mismatch at %d: %s != %s\n' "$i" "${actual[i]}" "${expected_args[i]}" >&2
            return 1
        fi
    done
    printf 'build:prod'
}
for invocation in vp vpr /path/vpr; do
    for style in separate attached equals; do
        case $style in
            separate) COMP_WORDS=("$invocation" -C 'apps\ web'); expected_index=4 ;;
            attached) COMP_WORDS=("$invocation" '-Capps\ web'); expected_index=3 ;;
            equals) COMP_WORDS=("$invocation" -C = 'apps\ web'); expected_index=3 ;;
        esac
        expected_args=(-- "$invocation")
        case $style in
            separate) expected_args+=(-C 'apps\ web') ;;
            attached) expected_args+=('-Capps\ web') ;;
            equals) expected_args+=('-C=apps\ web') ;;
        esac
        [[ $invocation != vp ]] || COMP_WORDS+=(run)
        expected_args+=(run build:pr)
        COMP_WORDS+=(build : pr)
        COMP_CWORD=$((${#COMP_WORDS[@]} - 1))
        COMP_LINE="${COMP_WORDS[*]}"
        COMP_LINE=${COMP_LINE//build : pr/build:pr}
        [[ $style != equals ]] || COMP_LINE=${COMP_LINE//-C = /-C=}
        _clap_complete_vp || exit 1
        [[ ${COMPREPLY[*]} == prod ]] || exit 1
    done
done
mkdir -p 'apps web' 'apps/web'
printf '{"name":"app","scripts":{"unique-task":"exit 0"}}\n' > 'apps web/package.json'
printf '{"name":"app","scripts":{"unique-task":"exit 0"}}\n' > 'apps/web/package.json'
for shell in bash zsh; do
    for dir in 'apps web' 'apps\ web' "'apps web'" '"apps web"' 'apps/"web"'; do
        result=$(VP_COMPLETE="$shell" _CLAP_COMPLETE_INDEX=4 "$vp_bin" -- vp -C "$dir" run unique)
        [[ $result == unique-task* ]] || exit 1
    done
done
mkdir -p "$HOME/home-project"
printf '{"name":"home-app","scripts":{"home-task":"exit 0"}}\n' > "$HOME/home-project/package.json"
result=$(VP_COMPLETE=bash _CLAP_COMPLETE_INDEX=4 "$vp_bin" -- vp -C '~/home-project' run home)
[[ $result == home-task* ]] || exit 1
rm -rf "$HOME/home-project"
rm -rf 'apps web' 'apps' completion.bash
printf 'completion argv, colon trimming, and escaped cwd: ok\n'
