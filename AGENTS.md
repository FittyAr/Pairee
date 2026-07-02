# Pairee Plugin Registry Branch

Esta es la rama de producción huérfana (orphan) **`plugin-registry`** del repositorio Pairee. Se utiliza exclusivamente para almacenar, publicar y distribuir el catálogo público de plugins de la comunidad.

## Estructura del Directorio

La estructura de esta rama está optimizada para particionar los plugins y evitar colisiones/cargas pesadas en carpetas únicas:

```text
plugin-registry/
├── .gitignore
├── AGENTS.md                           # Estas instrucciones
└── registry/
    ├── index.toml                      # Catálogo maestro con metadatos y hashes de integridad SHA-256
    └── plugins/                        # Carpeta raíz de los plugins
        └── <inicial_autor_minuscula>/  # Primera letra del nombre del autor (ej. 'f' para 'FittyAr', o '_' si no es a-z)
            └── <nombre_autor>/         # Carpeta exclusiva del autor (ej. 'FittyAr')
                └── <nombre_plugin>/    # Directorio del plugin en su última versión
                    ├── manifest.toml   # Manifiesto del plugin
                    ├── main.lua        # Archivo principal de ejecución
                    ├── sha256.sum      # Suma de verificación SHA-256 de los archivos
                    ├── help/           # Documentación de ayuda
                    ├── lang/           # Archivos de localización/idioma
                    └── screenshots/    # Capturas de pantalla e íconos requeridos
```

## Reglas para Agentes (AI Coding Assistants)

Cuando trabajes en esta rama o generes herramientas para interactuar con ella, asegúrate de cumplir las siguientes directrices:

1. **Ubicación de Plugins:** Todos los archivos de un plugin deben empaquetarse estrictamente en `registry/plugins/<inicial_autor_minuscula>/<nombre_autor>/<nombre_plugin>/`.
2. **Índice del Registro (`index.toml`):** Cada vez que se agregue o actualice un plugin, se debe actualizar la entrada correspondiente en `registry/index.toml` respetando la estructura serializada de `RegistryIndex`.
3. **No Eliminación (Sólo anexar):** No elimines plugins ni versiones anteriores del registro, ya que la base de datos de plugins es de historial acumulativo.
4. **Archivos Temporales:** Respeta el archivo `.gitignore` configurado en esta rama para evitar subir binarios (`target/`), carpetas de ejemplo local (`example/`), dependencias de Rust (`Cargo.lock`) o temporales de editores de código.
