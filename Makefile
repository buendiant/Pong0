# Makefile para Proyecto Pong Telemática (Cliente / Servidor)
# Cumple con los requerimientos estipulados en ProyectoPongN1.md

CARGO ?= cargo
PORT ?= 8080
LOG_FILE ?= server.log

.PHONY: all server client test check run-server run-client clean help

all: server client

# Compilación del Servidor Headless (sin dependencias gráficas, ideal para Linux / AWS EC2)
server:
	@echo "==> Compilando PongServer (Headless, sin dependencias gráficas)..."
	$(CARGO) build --release --bin server --no-default-features
	@# En Linux/Unix copiamos el binario a la raíz para permitir la ejecución: ./server <PORT> <Log File>
	@if [ -f target/release/server ]; then cp target/release/server ./server && chmod +x ./server; fi
	@if [ -f target/release/server.exe ]; then cp target/release/server.exe ./server.exe; fi
	@echo "==> Servidor compilado exitosamente."

# Compilación del Cliente Gráfico con Macroquad
client:
	@echo "==> Compilando PongClient (Interfaz Gráfica Macroquad)..."
	$(CARGO) build --release --bin client
	@if [ -f target/release/client ]; then cp target/release/client ./client && chmod +x ./client; fi
	@if [ -f target/release/client.exe ]; then cp target/release/client.exe ./client.exe; fi
	@echo "==> Cliente compilado exitosamente."

# Ejecución de la suite completa de 41 pruebas automatizadas
test:
	@echo "==> Ejecutando pruebas unitarias y de integración de red..."
	$(CARGO) test

# Verificación estática de tipos y linter Clippy
check:
	@echo "==> Verificando sintaxis y tipos..."
	$(CARGO) check --bin server --no-default-features
	$(CARGO) check --bin client
	$(CARGO) clippy --all-targets -- -D warnings

# Ejecutar el servidor con parámetros por defecto o personalizados: make run-server PORT=8080 LOG_FILE=pong.log
run-server: server
	@echo "==> Iniciando servidor en puerto $(PORT), guardando log en $(LOG_FILE)..."
	@if [ -f ./server ]; then ./server $(PORT) $(LOG_FILE); else $(CARGO) run --bin server --no-default-features -- $(PORT) $(LOG_FILE); fi

# Ejecutar el cliente gráfico (se puede pasar ARGS="<SERVER_IP:PORT> <NICK> <EMAIL>")
run-client: client
	@echo "==> Iniciando cliente gráfico..."
	@if [ -f ./client ]; then ./client $(ARGS); else $(CARGO) run --bin client -- $(ARGS); fi

# Limpieza de artefactos generados
clean:
	@echo "==> Limpiando artefactos de compilación..."
	$(CARGO) clean
	@rm -f ./server ./server.exe ./client ./client.exe *.log

help:
	@echo "Opciones disponibles en este Makefile:"
	@echo "  make all         - Compila tanto el servidor como el cliente en modo release"
	@echo "  make server      - Compila el binario headless 'server' (para AWS EC2 o local)"
	@echo "  make client      - Compila el cliente gráfico 'client'"
	@echo "  make test        - Ejecuta las 41 pruebas unitarias y de integración"
	@echo "  make check       - Ejecuta cargo check y clippy para validar el código"
	@echo "  make run-server  - Inicia el servidor (usa PORT=$(PORT) y LOG_FILE=$(LOG_FILE))"
	@echo "  make run-client  - Inicia el cliente gráfico"
	@echo "  make clean       - Elimina artefactos temporales y ejecutables copiados"
