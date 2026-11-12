#!/bin/sh
# Abre la AppImage de MNEMOS que está en la misma carpeta que este archivo,
# esté donde esté montado el disco. Uso: sh MNEMOS.sh
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 1
exec "$dir/MNEMOS-linux.AppImage" "$@"
