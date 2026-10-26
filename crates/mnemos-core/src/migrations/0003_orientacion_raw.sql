-- Hasta ahora no se leía la orientación de los RAW con contenedor propio
-- (CR3, RAF, ORF y RW2) y se guardaba 1. Se marca como no leída para que el
-- próximo indexado la lea, sin recalcular la huella.
UPDATE photos SET orientation = NULL
WHERE lower(rel_path) LIKE '%.cr3'
   OR lower(rel_path) LIKE '%.raf'
   OR lower(rel_path) LIKE '%.orf'
   OR lower(rel_path) LIKE '%.rw2';
