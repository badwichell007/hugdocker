# Shell Completions

生成补全脚本：

```bash
hugdocker completion bash > hugdocker.bash
hugdocker completion zsh > _hugdocker
hugdocker completion fish > hugdocker.fish
```

常见安装位置：

```bash
mkdir -p ~/.local/share/bash-completion/completions
hugdocker completion bash > ~/.local/share/bash-completion/completions/hugdocker

mkdir -p ~/.zfunc
hugdocker completion zsh > ~/.zfunc/_hugdocker

mkdir -p ~/.config/fish/completions
hugdocker completion fish > ~/.config/fish/completions/hugdocker.fish
```
