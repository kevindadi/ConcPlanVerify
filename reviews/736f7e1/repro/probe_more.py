from pathlib import Path
exec((Path(__file__).parent / 'probe.py').read_text().split("cv={'name':")[0])

sem=lambda n:{'name':n,'kind':'sync','type':'Semaphore','mode':'Sync','count':0}
mutex={'name':'m','kind':'sync','type':'Mutex','mode':'Sync'}
cv={'name':'cv','kind':'sync','type':'Condvar','mode':'Sync'}

test('nested_handle_false_pass',model('nested_handle_false_pass',[
    fun('main',[op('spawn',func='a',handle='h'),op('call',func='helper'),op('join',handle='h'),op('semaphore_release',resource='gate'),op('return')]),
    fun('helper',[op('spawn',func='b',handle='h'),op('join',handle='h'),op('return')]),
    fun('a',[op('semaphore_acquire',resource='gate'),op('return')],form='closure'),
    fun('b',[op('return')],form='closure')],[sem('gate')]))

test('notify_choice_false_pass',model('notify_choice_false_pass',[
    fun('main',[op('scope',funcs=['w1','w2','notifier']),op('return')]),
    fun('w1',[op('mutex_lock',resource='m'),op('semaphore_release',resource='g12'),op('condvar_wait',condvar='cv',lock='m'),op('condvar_notify_all',condvar='cv'),op('mutex_unlock',resource='m'),op('return')],form='closure'),
    fun('w2',[op('semaphore_acquire',resource='g12'),op('mutex_lock',resource='m'),op('semaphore_release',resource='gN'),op('condvar_wait',condvar='cv',lock='m'),op('mutex_unlock',resource='m'),op('return')],form='closure'),
    fun('notifier',[op('semaphore_acquire',resource='gN'),op('mutex_lock',resource='m'),op('condvar_notify',condvar='cv'),op('mutex_unlock',resource='m'),op('return')],form='closure')],
    [mutex,cv,sem('g12'),sem('gN')]))

test('invalid_protected_write_fixed_fixture',model('invalid_protected_write_fixed_fixture',[
    fun('main',[op('write_shared',resource='x',expr='1'),op('return')])],
    [mutex,{'name':'x','kind':'var','type':'Var','base':'Int','init':0}],
    [{'var':'x','lock':'m'}]))

test('unused_unsupported',model('unused_unsupported',[
    fun('main',[op('return')])],
    [{'name':'rw','kind':'sync','type':'RwLock','mode':'Sync'}]))

test('bodyless_entry',model('bodyless_entry',[
    fun('main',[],effects={'reads':[],'writes':[]},may_block=False)]))

test('runtime_invalid_exit',model('runtime_invalid_exit',[
    fun('main',[op('mutex_unlock',resource='m'),op('return')])],[mutex]))

# Renaming only the callee-local handle must not change program behavior.
p=json.loads((ROOT/'nested_handle_false_pass.json').read_text())
f=next(f for f in p['modules'][0]['functions'] if f['name']=='helper')
for s in f['body']:
    if s.get('handle')=='h': s['handle']='local_h'
test('nested_handle_renamed_control',p)
