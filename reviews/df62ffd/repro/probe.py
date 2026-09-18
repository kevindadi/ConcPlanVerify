import copy
import json
import os
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parent
REPO = pathlib.Path('/Users/kevin/local-repos/ConcIR')
BIN = pathlib.Path(os.environ.get('CONCIR_REVIEW_BIN', '/private/tmp/concir-audit-round2-target/debug/concir-backend'))

def save(name, obj):
    p = ROOT / (name + '.json')
    p.write_text(json.dumps(obj, indent=2) + '\n')
    return str(p)

def run(name, args):
    r = subprocess.run([str(BIN), *map(str,args)], capture_output=True, text=True, timeout=60)
    (ROOT / (name + '.stdout')).write_text(r.stdout)
    (ROOT / (name + '.stderr')).write_text(r.stderr)
    try: data = json.loads(r.stdout)
    except ValueError: data = {'raw': r.stdout, 'stderr': r.stderr}
    return {'exit': r.returncode, 'data': data}

def op(kind, **kw): return {'kind': kind, **kw}
def fun(name, body, **kw):
    return {'name': name, 'kind': 'normal', 'body': [dict(sid=f's{i+1}', **s) for i,s in enumerate(body)], **kw}
def model(name, funcs, resources=None, protection=None):
    return {'program':name, 'version':'3.5.0', 'entry':'main::main', 'modules':[
        {'name':'main', 'resources':resources or [], 'protection':protection or [], 'functions':funcs}]}

def test(name, p, contract=None):
    path=save(name,p)
    cp=save(name+'_contract', contract or {'name':name, 'properties':[{'kind':'deadlock_free','id':'deadlock'}], 'bounds':{'max_depth':40,'max_states':10000}})
    results={'check':run(name+'_check',['check',path])}
    for engine in ['interp','petri']:
        results[engine]=run(name+'_'+engine,['explore',path,cp,engine])
    brief={k:{'exit':v['exit'], **{key:v['data'].get(key) for key in ['valid','outcome','complete','states_explored','invalid'] if key in v['data']}} for k,v in results.items()}
    print(name, json.dumps(brief))
    return results



src={'name':'src','kind':'var','type':'Var','base':'Int','init':2}
x={'name':'x','kind':'var','type':'Var','base':'Int','init':0}
ret={'name':'r','type':{'Int':[0,1]},'modeled':True}
for dst in ['x','_']:
    call=op('call',func='two',args=[]) if dst=='_' else op('call',func='two',args=[],dst=dst)
    p=model('declared_return',[fun('main',[call,op('return')]),fun('two',[op('return',value='src')],returns=ret)], [src,x])
    c={'name':'return-contract','properties':[{'kind':'reachability','id':'completed','goal':{'kind':'function_completed','function':'main::two'}}]}
    test('r4_return_domain_'+('discard' if dst=='_' else 'wide_dst'),p,c)
p=model('entry_return_domain',[fun('main',[op('return',value='src')],returns=ret)],[src])
c={'name':'entry-return','properties':[{'kind':'reachability','id':'completed','goal':{'kind':'function_completed','function':'main::main'}}]}
test('r4_return_domain_entry',p,c)

# In-domain control: src=1 should complete.
p['modules'][0]['resources'][0]['init']=1
test('r4_return_domain_valid',p,c)
