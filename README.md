# flow

Temporizador Pomodoro y Flowmodoro para Linux, con un daemon en segundo plano y un cliente por línea de comandos.

## Requisitos

- Rust (edition 2024)
- `libdbus` (para notificaciones vía zbus)

## Instalación

```sh
cargo build --release
install -m 755 target/release/flow ~/.local/bin/flow
```

## Uso

```sh
# Iniciar un pomodoro (25m trabajo / 5m descanso por defecto)
flow start

# Iniciar con una tarea y duración personalizada
flow start --task "arreglar bugs" --work 50m --break 10m

# Iniciar un flowmodoro (descanso proporcional al tiempo trabajado, ratio 5:1)
flow start --flow

# Estado actual
flow status
flow status --json
flow status --waybar

# Pausar / reanudar
flow pause

# Cancelar la sesión actual
flow cancel

# Pedir descanso (solo flowmodoro)
flow break

# Temporizador (cuenta regresiva)
flow timer 10m
flow timer 30s --task "cocinar la pasta"

# Cronómetro (cuenta hacia arriba)
flow stopwatch
flow stopwatch --task "reunión"

# Detener el daemon
flow stop

# Ver / editar configuración
flow config
flow config --work 50m --break 10m

# Historial de sesiones
flow history
flow history --json
```

## Configuración

El archivo de configuración vive en `$XDG_CONFIG_HOME/flow/config.json` (o `~/.config/flow/config.json`). Los valores de `flow config` se usan como valores por defecto al iniciar una sesión:

- `work_duration_secs` — duración del trabajo (defecto 25m)
- `break_duration_secs` — duración del descanso (defecto 5m)
- `long_break_duration_secs` — descanso largo (defecto 15m)
- `sessions_before_long_break` — sesiones antes del descanso largo (defecto 4)
- `break_ratio` — ratio flowmodoro trabajo:descanso (defecto 5)
- `hooks` — comandos a ejecutar en los cambios de fase

### Hooks

Se pueden definir comandos que se ejecutan al inicio/fin de cada fase, tanto por CLI como en la configuración:

```sh
flow start --on-work-start "mpv notify.mp3" --on-work-end "notify-send 'Listo'"
```

## Notas

- El daemon usa un socket Unix en `$XDG_RUNTIME_DIR/flow/flow.sock` y se auto-reinicia si no está corriendo.
- Las notificaciones se envían de forma asíncrona por D-Bus; si no hay bus de sesión, se omiten silenciosamente.
