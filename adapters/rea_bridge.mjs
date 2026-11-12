// Read-only adapter for the installed REA 6.8.0 API. No setup or target execution.
import {readFile,open} from 'node:fs/promises';
import {createInterface} from 'node:readline';
import {pathToFileURL} from 'node:url';
import {join} from 'node:path';
const lines=createInterface({input:process.stdin});
let request, service;
const abort=new AbortController();
const first=new Promise(resolve=>lines.on('line',line=>{
  if(line.length>65536) throw Error('REA request exceeds 64 KiB');
  const value=JSON.parse(line);
  if(!request){request=value;resolve();}
  else if(value.id===request.id&&value.op==='cancel') abort.abort();
}));
const emit=(value)=>process.stdout.write(JSON.stringify({v:1,id:request.id,...value})+'\n');
try{
  await first;
  if(request.v!==1) throw Error('Unsupported DIVULGE REA protocol');
  const root=process.argv[2];
  const pkg=JSON.parse(await readFile(join(root,'package.json'),'utf8'));
  if(pkg.name!=='rea-agents'||pkg.version!=='6.8.0') throw Error('This adapter requires REA 6.8.0; select a compatible installation.');
  const load=(path)=>import(pathToFileURL(join(root,'dist',path)).href);
  const {ByteEntropyService}=await load('application/binaryDiagnostics/ByteEntropyService.js');
  const {parseEvidence,evidenceSemanticContent}=await load('domain/evidence.js');
  const {createEvidenceBundle,parseEvidenceBundle}=await load('domain/evidenceBundle.js');
  const {canonicalJson}=await load('domain/comparisonSemantics.js');
  const metadata={rea_version:pkg.version,capabilities:['inspect_byte_entropy','validate_entropy_bundle']};
  let bundle;
  if(request.op==='discover'){emit({kind:'ready',...metadata});}
  else{
    if(request.op==='entropy'){
      service=new ByteEntropyService();
      const result=await service.inspect({path:request.path,offset:request.offset,length:request.length,block_size:request.block_size},{signal:abort.signal});
      if(!result.ok) throw result.error;
      const evidence=parseEvidence(result.value);
      if(evidence.subject?.digest.sha256!==request.artifact_sha256) throw Error('Source changed or belongs to another artifact. Verify it again.');
      bundle=createEvidenceBundle([evidence]);
    }else if(request.op==='import_bundle'){
      const handle=await open(request.path,'r');
      let bytes=Buffer.alloc(1048577),count=0;
      try{
        if(!(await handle.stat()).isFile()) throw Error('Choose a regular REA bundle file');
        while(count<bytes.length){const result=await handle.read(bytes,count,bytes.length-count,count);if(result.bytesRead===0)break;count+=result.bytesRead;}
      }finally{await handle.close();}
      if(count>1048576) throw Error('REA bundle exceeds 1 MiB');
      bundle=parseEvidenceBundle(JSON.parse(bytes.subarray(0,count).toString('utf8')));
      if(bundle.unknowns.length||bundle.records.some(e=>e.operation!=='inspect_byte_entropy'||e.subject?.digest.sha256!==request.artifact_sha256)) throw Error('This adapter accepts entropy bundles for the current artifact only; other evidence remains unsupported, not empty.');
    }else throw Error('Unsupported REA operation');
    if(abort.signal.aborted) emit({kind:'cancelled',...metadata});
    else emit({kind:'completed',...metadata,bundle_json:JSON.stringify(bundle),semantic:bundle.records.map(e=>canonicalJson(evidenceSemanticContent(e)))});
  }
}catch(error){emit({kind:abort.signal.aborted?'cancelled':'error',error:String(error?.message||error).slice(0,2000)});}
finally{await service?.close();lines.close();}
