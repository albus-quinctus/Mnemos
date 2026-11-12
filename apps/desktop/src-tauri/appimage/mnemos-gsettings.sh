# Lo carga el AppRun de la AppImage antes de abrir MNEMOS (no se ejecuta solo).
#
# La GTK que va dentro de la AppImage aborta al arrancar en compositores Wayland
# sin escritorio completo (Sway, Hyprland, river...) cuando el tema de cursor de
# GSettings está vacío. En esos entornos GSettings no guarda ajustes que la app
# necesite, así que se usan los valores por defecto en memoria. En GNOME, KDE y
# demás escritorios que sí lo usan no se toca nada, y si el usuario ya ha fijado
# GSETTINGS_BACKEND, se respeta.
if [ -z "${GSETTINGS_BACKEND:-}" ] && [ -n "${WAYLAND_DISPLAY:-}" ]; then
  case "${XDG_CURRENT_DESKTOP:-}" in
    *GNOME* | *KDE* | *Cinnamon* | *XFCE* | *MATE* | *Budgie* | *Unity* | \
    *Pantheon* | *Deepin* | *COSMIC* | *LXQt* | *LXDE*) ;;
    *) export GSETTINGS_BACKEND=memory ;;
  esac
fi
