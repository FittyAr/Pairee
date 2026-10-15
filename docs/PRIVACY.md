Privacy Policy for Pairee

Last updated: October 2026

The Pairee application is committed to protecting the privacy of its users. This privacy policy describes the handling of information when using our command-line interface (CLI) tool.

1. Information Collection and Use
Pairee is a local application. It does not collect, store, transmit, or share any type of personal information, identification data, telemetry, or user usage statistics. All data processing is performed locally on the user's device.

2. Internet Connectivity
Pairee never sends telemetry, usage statistics or personal data. It does, however, contact the network in the following cases:
- Update check: unless disabled, at every start Pairee makes one request to the GitHub API (api.github.com, latest release of the Pairee repository) to learn whether a newer version exists. As with any HTTPS request, GitHub can see the IP address and the program's user-agent. To disable it, set `auto_update_check = false` in `settings.toml`. Downloading an update only happens when you accept it.
- Plugin registry: installing, checking or updating plugins reads the plugin index, manifests and block list from the project's registry branch on GitHub (raw.githubusercontent.com). This happens only when you use the plugin manager. Plugins you install may have their own network access, subject to their declared permissions.
- 7-Zip (Windows): when Pairee runs on Windows and the standalone 7-Zip console executable is not yet present, it downloads it in the background from the 7-Zip project's GitHub releases (github.com/ip7z/7zip), checked against a pinned SHA-256 hash, to support archive formats the built-in code does not handle. On Linux and macOS the system 7z is used and nothing is downloaded.
- Remote connections: FTP, SFTP/SSH, S3 and WebDAV connections are made only to the servers you configure and open yourself. Credentials are used only for that server.
- Git: git operations (fetch, pull, push, clone) run only on your request, against the remotes of your own repositories.

3. Changes to this Privacy Policy
We reserve the right to update this policy in the future if new features are introduced that require it. Any modifications will be published directly in this space or within the project's official repository.

For any questions or inquiries regarding the local operation of this tool, you may get in touch through the developer's official support channels.

-----------------

Política de Privacidad para Pairee

Última actualización: Octubre de 2026

La aplicación Pairee se compromete a proteger la privacidad de sus usuarios. Esta política de privacidad describe el tratamiento de la información al utilizar nuestra herramienta de línea de comandos (CLI).

1. Recopilación y Uso de Información
Pairee es una aplicación local. No recopila, almacena, transmite ni comparte ningún tipo de información personal, datos de identificación, telemetría o estadísticas de uso del usuario. Todo el procesamiento de datos se realiza de forma local en el dispositivo del usuario.

2. Conectividad a Internet
Pairee nunca envía telemetría, estadísticas de uso ni datos personales. Sin embargo, accede a la red en los siguientes casos:
- Búsqueda de actualizaciones: salvo que se desactive, en cada inicio Pairee hace una petición a la API de GitHub (api.github.com, última versión del repositorio de Pairee) para saber si existe una versión más nueva. Como en cualquier petición HTTPS, GitHub puede ver la dirección IP y el agente de usuario del programa. Para desactivarla, ponga `auto_update_check = false` en `settings.toml`. La descarga de una actualización solo ocurre si usted la acepta.
- Registro de plugins: instalar, comprobar o actualizar plugins lee el índice, los manifiestos y la lista de bloqueo desde la rama de registro del proyecto en GitHub (raw.githubusercontent.com). Solo ocurre cuando usa el gestor de plugins. Los plugins que instale pueden tener su propio acceso a la red, según los permisos que declaren.
- 7-Zip (Windows): cuando Pairee se ejecuta en Windows y el ejecutable de consola independiente de 7-Zip aún no está presente, lo descarga en segundo plano desde las versiones publicadas del proyecto 7-Zip en GitHub (github.com/ip7z/7zip), verificado con un hash SHA-256 fijado, para dar soporte a formatos de archivo que el código integrado no maneja. En Linux y macOS se usa el 7z del sistema y no se descarga nada.
- Conexiones remotas: las conexiones FTP, SFTP/SSH, S3 y WebDAV se hacen solo a los servidores que usted configura y abre. Las credenciales se usan únicamente para ese servidor.
- Git: las operaciones de git (fetch, pull, push, clone) se ejecutan solo a petición suya, contra los remotos de sus propios repositorios.

3. Cambios a esta Política de Privacidad
Nos reservamos el derecho de actualizar esta política en el futuro si se introducen nuevas funcionalidades que así lo requieran. Cualquier modificación será publicada directamente en este espacio o en el repositorio oficial del proyecto.

Para cualquier duda o consulta sobre el funcionamiento local de la herramienta, puede ponerse en contacto a través de los canales oficiales de soporte del desarrollador.
