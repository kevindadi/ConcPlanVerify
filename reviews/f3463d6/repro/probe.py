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


sem=lambda n,count=0:{'name':n,'kind':'sync','type':'Semaphore','mode':'Sync','count':count}
mutex=lambda n:{'name':n,'kind':'sync','type':'Mutex','mode':'Sync'}
cv={'name':'cv','kind':'sync','type':'Condvar','mode':'Sync'}
test('r2_multi_lock_cv',model('r2_multi_lock_cv',[
    fun('main',[op('scope',funcs=['w1','w2','notifier']),op('return')]),
    fun('w1',[op('mutex_lock',resource='m1'),op('semaphore_release',resource='ready'),op('condvar_wait',condvar='cv',lock='m1'),op('mutex_unlock',resource='m1'),op('return')],form='closure'),
    fun('w2',[op('mutex_lock',resource='m2'),op('semaphore_release',resource='ready'),op('condvar_wait',condvar='cv',lock='m2'),op('mutex_unlock',resource='m2'),op('return')],form='closure'),
    fun('notifier',[op('semaphore_acquire',resource='ready',count=2),op('mutex_lock',resource='m1'),op('mutex_lock',resource='m2'),op('condvar_notify_all',condvar='cv'),op('mutex_unlock',resource='m2'),op('mutex_unlock',resource='m1'),op('return')],form='closure')],
    [mutex('m1'),mutex('m2'),cv,sem('ready')]))

test('r2_scope_loop',model('r2_scope_loop',[
    fun('main',[op('scope',funcs=['worker']),op('goto',target='s1')]),
    fun('worker',[op('return')],form='closure')]))

test('r2_spawn_join_loop',model('r2_spawn_join_loop',[
    fun('main',[op('spawn',func='worker',handle='h'),op('join',handle='h'),op('goto',target='s1')]),
    fun('worker',[op('return')],form='closure')]))

test('r2_semaphore_overflow',model('r2_semaphore_overflow',[
    fun('main',[op('semaphore_release',resource='s'),op('return')])],[sem('s',9223372036854775807)]))

bug=json.loads((REPO/'examples/lockorder_bug.json').read_text())
base=json.loads((REPO/'examples/lockorder_contract.json').read_text())
base['allowed_scope']={'functions':['main::t1'],'allow_lock_reorder':True}
bp=save('r2_scope_bug',bug)
cp=save('r2_fqn_scope_contract',base)
pp=save('r2_scope_swap_patch',[{'id':'swap','module':'main','function':'t1','changes':[{'kind':'swap_statements','a':'s1','b':'s2'}]}])
print('r2_fqn_scope_file',json.dumps(run('r2_fqn_scope_file',['repair',bp,cp,pp])))
print('r2_fqn_scope_auto',json.dumps(run('r2_fqn_scope_auto',['repair',bp,cp])))


for resource_kind in ['Var','Atomic']:
    p=model('query_only_'+resource_kind,[fun('main',[op('return')])],[{'name':'x','kind':'var','type':resource_kind,'base':'Int','init':0}])
    for negate in [False,True]:
        pred={'kind':'var_eq','resource':'main::x','value':0}
        if negate: pred={'kind':'not','predicate':pred}
        test('r2_query_only_'+resource_kind+('_negated' if negate else ''),p,{'name':'query-only','properties':[{'kind':'safety','id':'value','invariant':pred}]})

p=model('default_module_used',[fun('main',[op('read_shared',resource='x',dst='_'),op('return')])],[{'name':'x','kind':'var','type':'Var','base':'Int','init':0}])
p['modules'].append({'name':'other','resources':[{'name':'x','kind':'var','type':'Var','base':'Int','init':1}], 'functions':[fun('unused',[op('read_shared',resource='x',dst='_'),op('return')])]})
for reverse in [False,True]:
    if reverse:p['modules'].reverse()
    test('r2_namespace_used_'+str(reverse),p,{'name':'entry-x','properties':[{'kind':'safety','id':'x-zero','invariant':{'kind':'var_eq','resource':'x','value':0}}]})

test('r2_bounded_dst_fixed',model('r2_bounded_dst',[
    fun('main',[op('atomic_load',resource='a',dst='x'),op('return')])],
    [{'name':'a','kind':'var','type':'Atomic','base':'Int','init':2},
     {'name':'x','kind':'var','type':'Var','base':{'Int':[0,1]},'init':0}]))

p=model('false_repair_query_only',[fun('main',[op('write_shared',resource='x',expr='0'),op('return')])],[{'name':'x','kind':'var','type':'Var','base':'Int','init':0}])
c={'name':'nonzero','properties':[{'kind':'safety','id':'nonzero','invariant':{'kind':'not','predicate':{'kind':'var_eq','resource':'main::x','value':0}}}], 'allowed_scope':{'allow_statement_delete':True}}
test('r2_query_only_repair_before',p,c)
bp=save('r2_query_only_repair_original',p)
cp=save('r2_query_only_repair_contract',c)
pp=save('r2_query_only_delete_patch',[{'id':'delete-write','module':'main','function':'main','changes':[{'kind':'delete_statement','sid':'s1'}]}])
print('r2_query_only_repair',json.dumps(run('r2_query_only_repair',['repair',bp,cp,pp])))
patched=copy.deepcopy(p)
patched['modules'][0]['functions'][0]['body'].pop(0)
test('r2_query_only_repair_after',patched,c)

p=model('bounded_dst_with_place', [fun('main',[op('atomic_load',resource='a',dst='x'),op('return')]), fun('unused',[op('read_shared',resource='x',dst='_'),op('return')])],[{'name':'a','kind':'var','type':'Atomic','base':'Int','init':2},{'name':'x','kind':'var','type':'Var','base':{'Int':[0,1]},'init':0}])
test('r2_bounded_dst_with_place',p,{'name':'out-of-domain','properties':[{'kind':'reachability','id':'forbidden','goal':{'kind':'not','predicate':{'kind':'or','predicates':[{'kind':'var_eq','resource':'main::x','value':0},{'kind':'var_eq','resource':'main::x','value':1}]}}}]})
