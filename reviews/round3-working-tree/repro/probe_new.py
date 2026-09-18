from pathlib import Path
exec((Path(__file__).parent/'old-repro/probe.py').read_text().split('sem=lambda')[0])

for cap in [0,1]:
    fs=[fun('main',[op('scope',funcs=['sender','receiver']),op('return')]),fun('sender',[op('channel_send',channel='c',value='a'),op('return')],form='closure'),fun('receiver',[op('channel_recv',channel='c',dst='_'),op('return')],form='closure')]
    test('r3_channel_domain_'+str(cap),model('channel_domain',fs,[{'name':'a','kind':'var','type':'Var','base':'Int','init':2},{'name':'c','kind':'sync','type':'Channel','mode':'Sync','base':{'Int':[0,1]},'capacity':cap}]))

A={'a':'X",b:"Y','b':'Z'}
B={'a':'X','b':'Y",b:"Z'}
# Read constant structs to avoid relying on the expression parser's escaping.
rs=[{'name':n,'kind':'var','type':'Var','base':{'Struct':{'a':'String','b':'String'}},'init':v} for n,v in [('x',{'a':'init','b':'init'}),('va',A),('vb',B)]]
p=model('canonical_collision',[fun('main',[op('scope',funcs=['w1','w2']),op('return')]),fun('w1',[op('read_shared',resource='va',dst='x'),op('return')],form='closure'),fun('w2',[op('read_shared',resource='vb',dst='x'),op('return')],form='closure')],rs)
for n,v in [('A',A),('B',B)]:
    c={'name':'two-final-values','properties':[{'kind':'reachability','id':'final-'+n,'goal':{'kind':'and','predicates':[{'kind':'scope_completed','function':'main::main','sid':'s1'},{'kind':'var_eq','resource':'main::x','value':v}]}}]}
    test('r3_canonical_collision_'+n,p,c)

# Nested bounded field through a load of a structurally-compatible value.
p=model('nested_bounded',[fun('main',[op('read_shared',resource='src',dst='x'),op('return')])],[{'name':'src','kind':'var','type':'Var','base':{'Struct':{'n':'Int'}},'init':{'n':2}},{'name':'x','kind':'var','type':'Var','base':{'Struct':{'n':{'Int':[0,1]}}},'init':{'n':0}}])
c={'name':'nested','properties':[{'kind':'reachability','id':'outside','goal':{'kind':'not','predicate':{'kind':'or','predicates':[{'kind':'var_eq','resource':'x','value':{'n':0}},{'kind':'var_eq','resource':'x','value':{'n':1}}]}}}]}
test('r3_nested_domain',p,c)

p=json.loads((ROOT/'r3_canonical_collision_A.json').read_text())
c=json.loads((ROOT/'r3_canonical_collision_A_contract.json').read_text())
goal=c['properties'][0]['goal']
c['properties']=[{'kind':'safety','id':'forbid-reachable-A','invariant':{'kind':'not','predicate':goal}}]
test('r3_canonical_false_pass',p,c)
