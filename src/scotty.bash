# chpwd hook
scotty_chpwd() {
    __SCOTTY__ add "$(pwd)" > /dev/null
}

case $PROMPT_COMMAND in
    *scotty*)
        ;;
    *)
        PROMPT_COMMAND="${PROMPT_COMMAND:+$(echo "${PROMPT_COMMAND}" | awk '{gsub(/; *$/,"")}1') ; }scotty_chpwd"
        ;;
esac

s() {
    local output="$(__SCOTTY__ search ${1})"
    if [[ -d "${output}" ]]; then
        if [[ -t 1 ]]; then # Use color if stdout is a terminal
            echo -e "\\033[31m${output}\\033[0m"
        else
            echo "${output}"
        fi
        cd "${output}"
    else
        false
    fi
}

_scotty() {
    OLDIFS=$IFS
    IFS=$'\n'
    COMPREPLY=( $(scotty search -a $2) )
    IFS=$OLDIFS
}

complete -F _scotty s
