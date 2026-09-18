import sys,json,tempfile,os
from pathlib import Path
sys.dont_write_bytecode=True
repo=Path('/Users/kevin/local-repos/ConcPlanVerify');sys.path.insert(0,str(repo/'python'))
from cir_workflow.concir_client import ConcirClient
from cir_workflow.offline_workflow import OfflineWorkflow
from cir_workflow.providers import ScriptedProvider
base=Path('/private/tmp/cpv-offline-v1-review');fixtures=repo/'python/tests/fixtures/programs'
class MissingArtifact(ConcirClient):
 def repair(self,*args,**kwargs):
  r=super().repair(*args,**kwargs)
  if r.artifact_path:Path(r.artifact_path).unlink()
  r.artifact_path=None
  return r
binary=Path('/Users/kevin/local-repos/ConcIR/target/release/concir-backend')
p=ScriptedProvider([{'text':(fixtures/'single_cycle.json').read_text()}]);w=OfflineWorkflow(MissingArtifact(binary,workdir=base/'missing/calls'),p,out_dir=base/'missing');r=w.run('two lock orders',json.loads((fixtures/'single_cycle_contract.json').read_text()))
probe={'missing_artifact':{'workflow_status':r.status,'repaired_by_tool':r.repaired_by_tool,'replay':r.replay,'error':r.error}}
stub=base/'bad-replay';stub.write_text('#!/bin/sh\nprintf garbage\n');stub.chmod(0o755);z=ConcirClient(stub,workdir=base/'badreplay').replay('{}');probe['invalid_replay_output']={'kind':z.kind,'status':z.status,'payload':z.payload}
probe['env_files_present']={name:(repo/name).is_file() for name in ['.env','env','.env.local']}
(base/'probes.json').write_text(json.dumps(probe,indent=2)+'\n');print(json.dumps(probe,indent=2))
