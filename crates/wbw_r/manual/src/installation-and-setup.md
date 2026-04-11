# Installation and Setup

## Install

```bash
R CMD INSTALL crates/wbw_r/r-package/whiteboxworkflows
```

## Smoke Test

```bash
Rscript -e 'library(whiteboxworkflows); s <- wbw_session(); cat(length(wbw_tool_ids(session = s)), "\n")'
```
