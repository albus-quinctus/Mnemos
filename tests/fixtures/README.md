# Imágenes de prueba

Solo imágenes propias o de dominio público, sin datos personales (EXIF con GPS eliminado).
Casos a cubrir: JPEG, PNG, HEIC, un RAW pequeño, EXIF roto, orientación rotada, nombres con caracteres especiales.

## Archivos
- `cmyk.jpg`: JPEG en CMYK de 16 × 16, generado, sin EXIF. Comprueba que las miniaturas de JPEG que no se pueden reducir al decodificar se generan igual.
- `fecha-exif.jpg`: JPEG de 16 × 16, generado, con solo `DateTimeOriginal = 2015:06:01 10:20:30` en el EXIF. Comprueba la fecha de captura (spec 004).
- `video-girado.mp4`: vídeo H.264 + AAC de 32 × 18, 1 s a 10 fps, generado con `ffmpeg` (`testsrc` y un tono), con giro de 90° y `creation_time = 2019-05-12T14:30:00Z`. Comprueba el lector de vídeos (spec 019).
- `video-camara.mov`: el mismo vídeo en MOV sin giro, con `creation_time = 2021-08-03T09:15:00Z`, `©mak = Pentax`, `©mod = K-70` y `©xyz = +43.1953-004.8547/` (coordenadas inventadas). Comprueba cámara y ubicación (spec 019).
