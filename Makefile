.DEFAULT_GOAL := help
.PHONY: help

COMMAND_LIST := ${MAKEFILE_LIST}

# if .env file exists, including secret infomations
ifneq ("$(wildcard .env)","")
include .env
endif

help:
	@grep -E '^[a-zA-Z].*:\s##\s.*$$' ${COMMAND_LIST} |sed 's/\\//g' |awk 'BEGIN {FS = ": ## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

get_git_tag:
	$(eval git_ver = $(shell git describe --tags --dirty --abbrev=0))
	$(eval git_ver_verbose = $(shell git describe --tags --dirty))

.PHONY: build-core
build-core: ## build core
	@wasm-pack build core --target web
