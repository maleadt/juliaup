# juliaup preview: build variants (`+nogpl`, `+opt`, ...)

A preview build of [juliaup](https://github.com/JuliaLang/juliaup) that can
install build variants of Julia:

- releases built without GPL libraries: `1.13+nogpl`, `release+nogpl`,
  `1.13.0-rc1+nogpl`, ...
- nightly variants: `nightly+opt`, `1.14-nightly+nogpl`, `nightly+assert`, ...

The binaries don't self-update. They are portable, so they don't touch an
existing juliaup installation.

## Install

Linux and macOS:

```sh
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)   TARGET=x86_64-unknown-linux-musl ;;
  Linux-aarch64)  TARGET=aarch64-unknown-linux-musl ;;
  Darwin-x86_64)  TARGET=x86_64-apple-darwin ;;
  Darwin-arm64)   TARGET=aarch64-apple-darwin ;;
esac
mkdir -p ~/.juliaup-preview && cd ~/.juliaup-preview
curl -fsSL https://github.com/maleadt/juliaup/releases/download/@TAG@/juliaup-@TAG@-$TARGET.tar.gz | tar xz
export JULIAUP_DEPOT_PATH=~/.juliaup-preview/depot
export PATH=~/.juliaup-preview:$PATH
```

Windows (PowerShell):

```powershell
mkdir $HOME\.juliaup-preview; cd $HOME\.juliaup-preview
curl.exe -fsSLO https://github.com/maleadt/juliaup/releases/download/@TAG@/juliaup-@TAG@-x86_64-pc-windows-gnu.zip
tar -xf juliaup-@TAG@-x86_64-pc-windows-gnu.zip
$env:JULIAUP_DEPOT_PATH = "$HOME\.juliaup-preview\depot"
$env:PATH = "$HOME\.juliaup-preview;$env:PATH"
```

Put the two variables in your shell profile to keep using the preview. Without
them, `julia` and `juliaup` are your regular installation again.

## Use

```sh
juliaup add 1.13+nogpl            # or release+nogpl, 1.13.0-rc1+nogpl, ...
julia +1.13+nogpl -e 'println(Base.USE_GPL_LIBS)'   # false

juliaup add nightly+opt           # `juliaup list` shows all nightly variants
julia +nightly+opt

juliaup update                    # moves 1.13+nogpl to the next 1.13.x
```

`nogpl` builds exist for Linux x86_64, macOS (x86_64 and aarch64) and Windows
x86_64. They are published separately from the standard builds, so a release
can lack one for a while; juliaup then reports, e.g.:

```
Error: No nogpl build of Julia 1.13.1 for linux/x86_64 is currently available.
```

## Uninstall

Remove `~/.juliaup-preview` and the two variables.
