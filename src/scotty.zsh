# We create 2 functions:
#   1. A function that we'll add to the chpwd hook
#   2. A shorthand for scotty

# chpwd hook
scotty_chpwd() {
    __SCOTTY__ add "$(pwd)" > /dev/null
}

typeset -gaU chpwd_functions
chpwd_functions+=(scotty_chpwd)

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
        echo "scotty: directory ${output} not found"
        false
    fi
}
