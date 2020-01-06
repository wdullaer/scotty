#!/bin/bash

# This script will output the commands for use in the example video
# They can be captured by asciinema using a pipe as follows:
# ./video.sh | asciinema rec --stdin -q -t "Scotty Introduction" introduction.cast

pv -qL15 <<< "# Scotty is a tool that allows you to jump to directories based on fuzzy matching"
sleep 1
pv -qL15 <<< "# First let's ensure scotty is installed"
sleep 1
pv -qL15 <<< "which scotty"
sleep 1
pv -qL15 <<< "# Next, add the shell wrapper scripts (you should add this to your .<shell>rc file)"
sleep 1
pv -qL15 <<< 'source <(scotty init zsh)'
sleep 1
pv -qL15 <<< "which s"
sleep 1
pv -qL15 <<< "# You need to visit the folder once, for scotty to know it exists"
sleep 1
pv -qL15 <<< "cd Downloads"
sleep 1
pv -qL15 <<< "cd"
sleep 1
pv -qL15 <<< "cd Documenten/dd-dns"
sleep 1
pv -qL15 <<< "cd"
sleep 1
pv -qL15 <<< "cd Documenten/scotty"
sleep 1
pv -qL15 <<< "# Now that there is some data, we can jump to these folders using abbreviations"
sleep 1
pv -qL15 <<< "s down"
sleep 1
pv -qL15 <<< "s hom"
sleep 1
pv -qL15 <<< "s ddd"
sleep 1
pv -qL15 <<< "s sc"
sleep 1
pv -qL15 <<< "# Using first the first letter of each subpath works too"
sleep 1
pv -qL15 <<< "cd"
sleep 1
pv -qL15 <<< "s ds"
sleep 1
pv -qL15 <<< "exit"