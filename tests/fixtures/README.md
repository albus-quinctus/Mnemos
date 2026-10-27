# Imágenes de prueba

Solo imágenes propias o de dominio público, sin datos personales (EXIF con GPS eliminado).
Casos a cubrir: JPEG, PNG, HEIC, un RAW pequeño, EXIF roto, orientación rotada, nombres con caracteres especiales.

## Archivos
- `cmyk.jpg`: JPEG en CMYK de 16 × 16, generado, sin EXIF. Comprueba que las miniaturas de JPEG que no se pueden reducir al decodificar se generan igual.
- `fecha-exif.jpg`: JPEG de 16 × 16, generado, con solo `DateTimeOriginal = 2015:06:01 10:20:30` en el EXIF. Comprueba la fecha de captura (spec 004).
