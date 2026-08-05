"""Print per-point summary of results/scaling_llm.json (generate + codegen legs)."""
import json
from pathlib import Path

data = json.loads(Path("results/scaling_llm.json").read_text(encoding="utf-8"))
for r in data["records"]:
    g, c, gm = r["generate"], r["codegen"], r["gold_cir_metrics"]
    gen_stmts = (g.get("cir_metrics") or {}).get("statement_count")
    loc = (c.get("code_metrics") or {}).get("loc")
    print(
        f"{r['pattern']} {r['threads']}x{r['size']}"
        f" | gold_stmts={gm['statement_count']}"
        f" | gen ok={g['success']} safe={g['verified_safe']} rounds={g['rounds']}"
        f" tok={g['input_tokens'] + g['output_tokens']} states={g.get('state_count')}"
        f" gen_stmts={gen_stmts}"
        f" | cg src={c['source']} ok={c['success']} rounds={c['rounds']}"
        f" tok={c['input_tokens'] + c['output_tokens']} loc={loc}"
    )
