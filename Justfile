# just manual: https://github.com/casey/just

_list:
	just --list

build:
	#!/bin/bash -eux
	docker build --build-arg GIT_REVISION=$(git rev-parse HEAD) -t fasterthanlime.registry.cpln.io/refresh:latest .

push:
	just build
	docker push fasterthanlime.registry.cpln.io/refresh:latest

run:
	just build
	docker run -p 8000:8000/tcp --rm \
		--env SERVE_MODE=SERVE_FRESH \
		--env DATABASE_URL=postgres://amos@localhost:5432/amos \
		--network host \
		fasterthanlime.registry.cpln.io/refresh:latest