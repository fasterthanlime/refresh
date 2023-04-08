# just manual: https://github.com/casey/just

_list:
	just --list

build:
	#!/bin/bash -eux
	docker build --build-arg GIT_REVISION=$(git rev-parse HEAD) -t fasterthanlime.registry.cpln.io/refresh:latest .

push:
	just build
	docker push fasterthanlime.registry.cpln.io/refresh:latest