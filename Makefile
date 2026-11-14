.PHONY: docs-check graphify

docs-check:
	python3 tools/check_docs.py

graphify:
	./tools/graphify.sh
