import copy
import json
import os
import pathlib
import subprocess

ROOT = pathlib.Path(__file__).resolve().parent
REPO = pathlib.Path('/Users/kevin/local-repos/ConcIR')
BIN = pathlib.Path(os.environ.get('CONCIR_REVIEW_BIN', '/private/tmp/concir-audit-736f7e1-target/debug/concir-backend'))

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

cv={'name':'cv','kind':'sync','type':'Condvar','mode':'Sync'}
test('notify_all_without_wait',model('notify_all_without_wait',[fun('main',[op('condvar_notify_all',condvar='cv'),op('return')])],[cv]))
test('nested_handle_names',model('nested_handle_names',[
    fun('main',[op('spawn',func='a',handle='h'),op('call',func='helper'),op('join',handle='h'),op('return')]),
    fun('helper',[op('spawn',func='b',handle='h'),op('join',handle='h'),op('return')]),
    fun('a',[op('return')],form='closure'),fun('b',[op('return')],form='closure')]))
test('finite_call_loop',model('finite_call_loop',[
    fun('main',[op('call',func='helper'),op('goto',target='s1')]),fun('helper',[op('return')])]))
test('invalid_protected_write',model('invalid_protected_write',[
    fun('main',[op('write_shared',resource='x',expr='1'),op('return')])],
    [{'name':'m','kind':'sync','type':'Mutex','mode':'Sync'},{'name':'x','kind':'var','type':'Var','base':'Int','init':0}],
    [{'var':'x','lock':'m'}] ))
test('ignored_assumptions',model('ignored_assumptions',[fun('main',[op('return')])]),{
    'name':'unsupported-semantics','properties':[{'kind':'deadlock_free','id':'deadlock'}],
    'assumptions':{'sequential_consistency':False,'no_spurious_wakeups':False}})

bug=json.loads((REPO/'examples/lockorder_bug.json').read_text())
base=json.loads((REPO/'examples/lockorder_contract.json').read_text())
patch=[{'id':'delete-three','module':'main','function':'t2','changes':[{'kind':'delete_statement','sid':s} for s in ['s1','s3','s5']]}]
bp=save('repair_original',bug)
pp=save('delete_patch',patch)
c=copy.deepcopy(base)
c['allowed_scope']={'allow_lock_reorder':False,'allow_statement_delete':False}
cp=save('forbidden_delete_contract',c)
r=run('forbidden_delete',['repair',bp,cp,pp])
print('forbidden_delete',json.dumps(r))
c=copy.deepcopy(base)
c['allowed_scope']={'allow_statement_delete':True}
c['preserved'].append({'kind':'reachable','description':'required s3 write must remain','goal':{'kind':'statement_reached','function':'main::t2','sid':'s3'}})
cp=save('stable_sid_contract',c)
r=run('stable_sid_deleted',['repair',bp,cp,pp])
print('stable_sid_deleted',json.dumps(r))
patched=copy.deepcopy(bug)
f=next(f for f in patched['modules'][0]['functions'] if f['name']=='t2')
f['body']=[s for s in f['body'] if s['sid'] not in ['s1','s3','s5']]
p=save('deleted_program',patched)
print('fresh_resolve_deleted_sid', json.dumps(run('fresh_resolve_deleted_sid',['explore',p,cp])))
c=copy.deepcopy(base)
c['allowed_scope']={'allow_lock_reorder':False}
cp=save('forbidden_reorder_contract',c)
pp=save('swap_patch',[{'id':'swap','module':'main','function':'t1','changes':[{'kind':'swap_statements','a':'s1','b':'s2'}]}])
print('forbidden_reorder',json.dumps(run('forbidden_reorder',['repair',bp,cp,pp])))
