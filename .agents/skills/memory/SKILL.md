---
name: memory
description: Registra y actualiza automáticamente los últimos avances del proyecto en MEMORY.md siguiendo la progresión Alpha (+0.1), BETA (MVP jugable) y v1 (interfaz completa).
---

# Procedimiento de Registro de Memoria (/memory)

Este comando es un alias de `/guardar-memoria` para automatizar el registro de avances técnicos en `MEMORY.md`.

## Pasos de Ejecución

1. **Inspección de Cambios Recientes:**
   * Ejecutar `git status` y `git diff` (o revisar el último commit con `git log -n 1`) para identificar qué archivos y componentes han sido añadidos, modificados o refinados.
   * Revisar los cambios en el código (`src/`, `Cargo.toml`, documentación, pruebas, etc.).

2. **Determinación de la Versión:**
   * Leer `MEMORY.md` para determinar la última versión registrada.
   * Aplicar las reglas de ciclo de vida del proyecto:
     * **Alpha:** Si el proyecto está en desarrollo iterativo previo a un juego completamente jugable, incrementar en **+0.1** (ej. de `Alpha 0.1` a `Alpha 0.2`, `Alpha 0.3`, etc.).
     * **BETA:** Transicionar a `BETA` **única y exclusivamente** si el Pong ya es funcionalmente jugable entre dos clientes y representa un MVP (Minimum Viable Product).
     * **v1:** Transicionar a `v1` (o `v1.0`) cuando la interfaz de usuario esté completa, pulida y el juego esté 100% jugable y listo para entrega final según `ProyectoPongN1.md`.

3. **Actualización de `MEMORY.md`:**
   * Insertar una nueva entrada en la sección **Historial de Versiones** de `MEMORY.md` con:
     * Encabezado con la nueva versión y la fecha actual (`YYYY-MM-DD`).
     * **Estado:** Actual (marcar la versión anterior como completada si aplica).
     * **Hito:** Breve resumen de 1 línea del objetivo alcanzado.
     * **Descripción técnica:** Lista con viñetas detallando la implementación, decisiones de red/Rust y cambios clave.
     * **Archivos intervenidos:** Lista de rutas modificadas o creadas.
     * **Próximos pasos pendientes:** Tareas inmediatas a desarrollar según la hoja de ruta y `ProyectoPongN1.md`.

4. **Confirmación al Usuario:**
   * Informar al usuario que la memoria fue actualizada con éxito, mostrando la versión asignada y el resumen de lo registrado.
