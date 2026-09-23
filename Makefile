.PHONY: supplement test
FREEZE_TAG ?= experiments-v2-freeze-7
CONCIR_TAG ?= concir-freeze-6

supplement:
	python3 scripts/make_supplement.py --tag $(FREEZE_TAG) --concir-tag $(CONCIR_TAG)

test:
	PYTHONPATH=python:. python3 -m unittest discover -s python/tests -t .
	cd ../ConcIR && cargo test --offline

verify-evidence:
	bash scripts/verify_evidence.sh
