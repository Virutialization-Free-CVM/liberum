.PHONY: build run submodule-update status

build:
	./scripts/build_liberum_all.sh

run:
	./scripts/run_tellus_wasmrt.sh

submodule-update:
	git submodule update --init --recursive

status:
	git status --short
	git submodule status
