# Pasar el disco a exFAT

macOS no puede escribir en discos NTFS. Para usar MNEMOS en Windows, macOS y
Linux, el disco tiene que estar en exFAT. Formatear **borra todo el disco**,
así que primero se copia todo a otro sitio, se formatea y se vuelve a copiar.

Esta guía es para Linux (Debian), con el disco `BACKUP_FOTOS`. Tarda alrededor
de una hora por cada 25 GB en un disco USB 2.0.

> **Antes de empezar:** usa otro cable o un puerto con más alimentación. Si el
> disco se desconecta a mitad de la copia, se repite el paso, pero si se
> desconecta mientras formateas hay que empezar el formateo de nuevo. exFAT no
> tiene diario: una desconexión con archivos abiertos puede dañar lo que se
> estaba escribiendo. Expulsa siempre el disco antes de desenchufarlo.

## 1. Comprobar el espacio libre

Hace falta espacio en el ordenador para todo lo que hay en el disco:

```
du -sh /media/lilith/BACKUP_FOTOS
df -h ~
```

El segundo número (columna «Disp») tiene que ser mayor que el primero.

## 2. Copiar todo al ordenador

Cierra MNEMOS antes. Luego:

```
mkdir -p ~/copia-BACKUP_FOTOS
rsync -a --info=progress2 /media/lilith/BACKUP_FOTOS/ ~/copia-BACKUP_FOTOS/
```

Comprueba que la copia es idéntica (no debe mostrar ninguna línea):

```
rsync -a --checksum --dry-run --itemize-changes /media/lilith/BACKUP_FOTOS/ ~/copia-BACKUP_FOTOS/
```

## 3. Formatear el disco en exFAT

Con la aplicación **Discos** (GNOME): elige el disco TOSHIBA, la partición
`BACKUP_FOTOS`, el engranaje → «Formatear partición…», tipo «Otro» → «exFAT»,
nombre `BACKUP_FOTOS`.

O en una terminal (comprueba antes con `lsblk -f` que la partición es la del
TOSHIBA, porque el nombre `sdX1` cambia cada vez que se conecta):

```
sudo apt install exfatprogs
lsblk -f
udisksctl unmount -b /dev/sdX1
sudo mkfs.exfat -L BACKUP_FOTOS /dev/sdX1
udisksctl mount -b /dev/sdX1
```

## 4. Volver a copiar

```
rsync -rt --info=progress2 ~/copia-BACKUP_FOTOS/ /media/lilith/BACKUP_FOTOS/
```

(`-rt` en vez de `-a`: exFAT no guarda dueños ni permisos de Linux.)

Comprueba de nuevo que todo es idéntico (no debe mostrar ninguna línea):

```
rsync -rt --checksum --dry-run --itemize-changes ~/copia-BACKUP_FOTOS/ /media/lilith/BACKUP_FOTOS/
```

## 5. Comprobar la biblioteca

```
mnemos info /media/lilith/BACKUP_FOTOS/MNEMOS
mnemos index /media/lilith/BACKUP_FOTOS/MNEMOS
```

`index` debe decir que todas las fotos están «sin cambios». Si es así, puedes
borrar `~/copia-BACKUP_FOTOS`. Si no, no borres la copia y avisa.
