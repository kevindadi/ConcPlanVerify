# Normalizer regression (G-2)

Historical `check_invalid`/`sid_invalid`/`schema_error` candidate texts re-run through the upgraded normalizer (sid fill/rename + field/expr/base repair): **3/13 valid** after normalization; the remaining ones carry semantic errors (undeclared names, type mismatches) that are not syntax-repairable and are returned as feedback.
