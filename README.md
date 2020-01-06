# Scotty
Scotty uses full text search techniques to rapidly go to directories in your shell that you have visited previously. It is implemented in rust, because I wanted to learn the language, but also to minize any latency so that your shell remains snappy.

[![asciicast](https://asciinema.org/a/BmxfCm6RNf4iCX1hhuzYKZPzf.svg)](https://asciinema.org/a/BmxfCm6RNf4iCX1hhuzYKZPzf)

## Installation
`scotty` can be installed in a few different ways. More will be added in the future if it gains any traction.

1. Install the binary

   #### Using Cargo
   If you have a working rust toolchain installed, `scotty` can easily be installed using cargo.

   ```sh
   cargo install scotty
   ```

   #### Download from github
   Download the latest release from http://github.com/wdullaer/scotty/releases and extract it to a place on your path

2. Add the init script to your shell's config file:

   #### Zsh
   Add the following to the end of your `~/.zshrc` file

   ```sh
   source <(scotty init zsh)
   ```

   #### Bash
   Add the following to the end of your `~/.bashrc` file

   ```sh
   source <(scotty init bash)
   ```

## Inspiration
The following projects have been an inspiration for various components in this project:
* [Autojump](https://github.com/wting/autojump): Provides similar functionality, implemented in python
* [Starship](https://starship.rs): A shell prompt implemented in rust
* [Meilisearch](https://www.meilisearch.com/): A full text search server implemented in rust
* [SublimeText Fuzzy Match](https://www.forrestthewoods.com/blog/reverse_engineering_sublime_texts_fuzzy_match/): A reverse engineering of Sublime Text's fuzzy match on files and directories

## License
All the code in this repository is released under the _Mozilla Public License v2.0_, for more information take a look at the [LICENSE](LICENSE) file.