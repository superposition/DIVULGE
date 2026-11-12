// Synthetic, public fixtures only. Usage: node generate.mjs <installed rea-agents root>
// REA computes the statistics; paths are replaced and semantic IDs recomputed by
// REA for these teaching fixtures. These are not receipts from the user's firmware.
import {mkdtemp,writeFile,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {pathToFileURL,fileURLToPath} from 'node:url';
const root=process.argv[2], load=p=>import(pathToFileURL(join(root,'dist',p)).href);
const {ByteEntropyService}=await load('application/binaryDiagnostics/ByteEntropyService.js');
const {evidenceSemanticContent,parseEvidence}=await load('domain/evidence.js');
const {digestCanonicalValue}=await load('domain/canonicalDigest.js');
const {canonicalJson}=await load('domain/comparisonSemantics.js');
const {createEvidenceBundle,parseEvidenceBundle}=await load('domain/evidenceBundle.js');
const pkg=JSON.parse(await readFile(join(root,'package.json'),'utf8'));
if(pkg.version!=='6.8.0')throw Error('Fixtures require REA 6.8.0');
const temp=await mkdtemp(join(tmpdir(),'divulge-rea-fixture-'));
const service=new ByteEntropyService();
try{
  const path=join(temp,'synthetic.bin');
  await writeFile(path,Buffer.concat([Buffer.alloc(256),Buffer.from(Array.from({length:256},(_,i)=>i))]));
  for(const length of [512,0]){
    const result=await service.inspect({path,offset:0,length,block_size:256});
    if(!result.ok)throw result.error;
    const e=JSON.parse(JSON.stringify(result.value));
    const portable='C:/divulge-fixtures/synthetic.bin';
    e.subject.local_path=portable;e.parameters.path=portable;e.normalized_result.artifact.path=portable;e.locations=[{kind:'artifact-path',path:portable}];
    e.evidence_id='ev_'+digestCanonicalValue(evidenceSemanticContent(e));parseEvidence(e);
    const bundle=createEvidenceBundle([e]);parseEvidenceBundle(bundle);
    const receipt={v:1,id:'synthetic-rea-'+length,kind:'completed',rea_version:'6.8.0',capabilities:['inspect_byte_entropy','validate_entropy_bundle'],bundle_json:JSON.stringify(bundle),semantic:[canonicalJson(evidenceSemanticContent(e))],error:''};
    await writeFile(fileURLToPath(new URL(`./entropy-${length}.json`,import.meta.url)),JSON.stringify(receipt,null,2)+'\n');
  }
}finally{await service.close();await rm(temp,{recursive:true});}
