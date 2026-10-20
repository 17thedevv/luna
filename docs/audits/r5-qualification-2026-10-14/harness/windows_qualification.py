"""Exact-pin Windows acquisition for the final implementation pin (R5_IMPL_FINAL).

This runner is outside the tested checkout. Immutable labels: a stage result file
is never overwritten, so a re-run must use a new label directory.
"""
from pathlib import Path
import argparse, datetime, gzip, hashlib, json, os, platform, re, shutil, struct, subprocess, sys, time

PIN = os.environ['R5_TESTED_COMMIT']
assert re.fullmatch(r'[0-9a-f]{40}', PIN), 'Expected immutable full SHA'
URL = 'https://github.com/17thedevv/luna.git'
TOP = Path(os.environ.get('R5_WINDOWS_OUT', 'D:/r5-'+PIN[:8]))
SRC = TOP/'windows-checkout'
OUT = TOP/'windows'
BUILD = Path(os.environ.get('R5_WINDOWS_BUILD', 'C:/r5-'+PIN[:8]+'/windows-build'))
CARGO = Path('C:/Users/84387/.cargo/bin/cargo.exe')
RUSTC = CARGO.with_name('rustc.exe')
LLVM = Path('D:/Programs/LLVM-DEV')
ENV = os.environ.copy()
ENV['R5_TESTED_COMMIT'] = PIN
ENV['PATH'] = str(CARGO.parent)+';'+str(LLVM/'bin')+';C:/mingw64/bin;'+ENV['PATH']
ENV.update(LLVM_SYS_180_PREFIX=str(LLVM), LLVM_CONFIG_PATH=str(LLVM/'bin/llvm-config.exe'), LLVM_HOME=str(LLVM),
    CARGO_TARGET_DIR=str(BUILD/'cargo-target'), LUNA_SYSROOT=str(SRC/'luna-rs'),
    LUNA_RUNTIME_LIB=str(BUILD/'runtime/libluna-runtime.a'))
CLI = BUILD/'cargo-target/debug/luna.exe'
RELEASE = BUILD/'cargo-target/release/luna.exe'

def utc(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
def sha(p): return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def save(p, data):
    p.parent.mkdir(parents=True,exist_ok=True)
    with p.open('w',encoding='utf-8',newline='\n') as f: json.dump(data,f,indent=2,ensure_ascii=False)
def event(kind, **data):
    data=dict(time=utc(),event=kind,**data)
    with (OUT/'events.jsonl').open('a',encoding='utf-8') as f:f.write(json.dumps(data)+'\n')
    print(json.dumps(data),flush=True)
def git(*args): return subprocess.check_output(['git',*args],cwd=SRC,env=ENV,text=True).strip()
def identity():
    assert git('rev-parse','HEAD')==PIN
    assert not git('diff','HEAD','--').strip(),'Tracked source changed'
    assert not git('diff','--cached').strip(),'Index changed'
def run(label,args,cwd=None,required=True,extra_env=None):
    result_file=OUT/(label+'.json')
    assert not result_file.exists(),'Immutable acquisition label already exists: '+label
    log=OUT/(label+'.log');log.parent.mkdir(parents=True,exist_ok=True)
    local=ENV.copy();local.update(extra_env or {})
    check_pin = (SRC/'.git').is_dir() and not label.startswith('acquisition-')
    if check_pin: identity()
    start=utc();tick=time.monotonic()
    event('START',label=label,args=list(map(str,args)))
    with log.open('wb') as stream:
        result=subprocess.run(list(map(str,args)),cwd=cwd or SRC,env=local,stdout=stream,stderr=subprocess.STDOUT)
    data=dict(tested_commit=PIN,label=label,command=list(map(str,args)),cwd=str(cwd or SRC),start_utc=start,end_utc=utc(),
        elapsed_seconds=round(time.monotonic()-tick,3),exit=result.returncode,log=log.relative_to(OUT).as_posix(),log_sha256=sha(log))
    if label.startswith('workspace-'):
        raw=log.read_text(encoding='utf-8',errors='replace')
        totals=re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored;',raw)
        data.update(result_groups=len(totals),passed=sum(int(x[0]) for x in totals),failed=sum(int(x[1]) for x in totals),ignored=sum(int(x[2]) for x in totals),binaries=len(re.findall(r'^\s+Running (?:unittests|tests[\\/])',raw,re.M)))
        gz=log.with_suffix('.log.gz');gz.write_bytes(gzip.compress(log.read_bytes(),mtime=0));data.update(log_archive=gz.name,archive_sha256=sha(gz))
    save(result_file,data)
    if check_pin:identity()
    event('END',label=label,exit=result.returncode,seconds=data['elapsed_seconds'])
    if required and result.returncode:raise RuntimeError(f'{label} failed: {result.returncode}')
    return data

def acquire(resume_clone=False):
    if SRC.exists():
        assert resume_clone,'Fresh clone path already exists; do not overwrite'
        assert json.loads((OUT/'acquisition-clone.json').read_text())['exit']==0
        assert git('remote','get-url','origin')==URL
        event('RESUME_FRESH_CLONE',reason='Clone completed; acquisition recorder originally checked pin before detach. No qualification had started.')
    else:
        run('acquisition-clone',['git','-c','core.longpaths=true','clone','--no-checkout',URL,SRC],cwd=TOP)
    run('acquisition-detach',['git','-c','core.longpaths=true','checkout','--detach',PIN],cwd=SRC)
    subprocess.run(['git','config','core.longpaths','true'],cwd=SRC,check=True)
    identity();assert not git('status','--porcelain'),'Initial checkout is not clean'
    versions={}
    for label,args in [('rustc',[RUSTC,'-vV']),('llvm',[LLVM/'bin/llvm-config.exe','--version']),('gcc',['gcc','-dumpmachine']),('cmake',['cmake','--version']),('ninja',['ninja','--version'])]:
        run('provenance-'+label,args)
        versions[label]=(OUT/('provenance-'+label+'.log')).read_text(encoding='utf-8',errors='replace')
    assert not list((SRC/'luna-rs/libs/external').rglob('*.llib')),'Fresh source clone has ignored canonical artifacts'
    save(OUT/'provenance.json',dict(tested_commit=PIN,remote_url=git('remote','get-url','origin'),head=git('rev-parse','HEAD'),initial_status=git('status','--porcelain'),
        os=platform.platform(),machine=platform.machine(),tools=versions,source_tree_objects={p:git('rev-parse','HEAD:'+p) for p in ['luna-rs','runtime','tests']},
        env={k:ENV[k] for k in ['LLVM_SYS_180_PREFIX','LLVM_CONFIG_PATH','LLVM_HOME','CARGO_TARGET_DIR','LUNA_SYSROOT','LUNA_RUNTIME_LIB']},
        note='Fresh remote clone at the final implementation pin; output/build directories outside checkout',time=utc()))

def verify_sysroot():
    import tomllib
    root=SRC/'luna-rs/libs/external'
    manifest=tomllib.loads((root/'sysroot.toml').read_text(encoding='utf-8'))
    providers=[]
    for provider in manifest['provider']:
        lib=root/(provider['path']+'.llib');obj=lib.with_suffix('.obj')
        data=lib.read_bytes();assert data[:4]==b'LLIB'
        assert struct.unpack('<HHH',data[4:10])==(2,25,5)
        target=data[10:74].split(b'\0')[0].decode();assert target=='x86_64-pc-windows-gnu', target
        assert struct.unpack('<H',obj.read_bytes()[:2])[0]==0x8664
        published=json.loads(subprocess.check_output([str(CLI),'manifest',str(lib)],cwd=SRC,env=ENV,text=True))
        contract=published['target']
        assert contract['target_triple']==target and contract['cpu']=='generic' and contract['features']==''
        assert contract['object_format']=='COFF' and contract['pointer_width']==64
        assert published['object_metadata']['size']==obj.stat().st_size
        assert bytes(published['object_metadata']['hash']).hex()==sha(obj)
        providers.append(dict(name=provider['name'],path=provider['path'],library_sha256=sha(lib),object_sha256=sha(obj),target=target,coff_machine='0x8664',cpu=contract['cpu'],features=contract['features'],abi=contract['abi']))
    assert len(providers)==len(list(root.rglob('*.llib')))==49
    versions=(SRC/'luna-rs/crates/luna-llib/src/format.rs').read_text()
    assert re.search(r'const SEMANTIC_METADATA_VERSION: u16 = 9;',versions)
    assert re.search(r'const LLIB_COMPILER_VERSION: u16 = 25;',versions)
    save(OUT/'sysroot-provenance.json',dict(tested_commit=PIN,compiler=25,metadata=9,mvir=5,format=2,provider_count=49,providers=providers,cli_debug_sha256=sha(CLI),cli_release_sha256=sha(RELEASE),runtime_sha256=sha(BUILD/'runtime/libluna-runtime.a')))

def build():
    BUILD.mkdir(parents=True,exist_ok=True)
    assert not (BUILD/'cargo-target').exists(),'Fresh build target already exists'
    run('runtime-config',['cmake','-S',SRC/'runtime','-B',BUILD/'runtime','-G','Ninja','-DCMAKE_BUILD_TYPE=Release','-DCMAKE_C_COMPILER=gcc','-DCMAKE_CXX_COMPILER=g++'])
    run('runtime-build',['cmake','--build',BUILD/'runtime','--parallel'])
    run('runtime-abi',['ctest','--test-dir',BUILD/'runtime','-R','^RuntimeAbiTests$','--output-on-failure','--no-tests=error'])
    run('cli-debug',[CARGO,'build','--locked','-p','luna-cli'],cwd=SRC/'luna-rs')
    run('cli-release',[CARGO,'build','--locked','-p','luna-cli','--release'],cwd=SRC/'luna-rs')
    run('sysroot',[CLI,'build-sysroot','--quiet'])
    verify_sysroot()
    save(OUT/'build-summary.json',dict(tested_commit=PIN,status='PASS',cli_debug_sha256=sha(CLI),cli_release_sha256=sha(RELEASE),runtime_sha256=sha(BUILD/'runtime/libluna-runtime.a'),time=utc()))

def focused():
    batches={
        'ownership':['generic_drop_cli','async_cleanup_cli','async_provider_export_cli'],
        'casts-views':['cast_contract_cli','cast_poison_cli','raw_slice_view_a5_cli','raw_slice_provenance_cli'],
        'calls':['named_arguments_cli','default_arguments_cli','default_omission_cli','call_arity_cli'],
        'traits-comptime':['impl_header_metadata_cli','projection_fallback_cli','comptime_dependency_precision_cli','comptime_loans_cli'],
        'diagnostics':['diagnostic_conformance_cli','expression_span_precision_cli','comptime_diagnostic_cascade_cli','compiler_trust_diagnostics_cli_parity','diagnostic_declaration_locations_cli','diag_per_code_coverage_cli','missing_return_cli','move_origin_join_cli'],
        'providers':['provider_tiers_cli','retained_contracts_cli','alpha_modules_cli','module_const_storage_cli','fresh_artifact_classification_cli','native_math_link_cli'],
        'arithmetic':['checked_arithmetic_qualification_cli','wrapping_arithmetic_cli','range_inclusive_cli'],
        'grammar-harness':['v01_grammar_acceptance_tests','test_temp_workspace_lifecycle_cli'],
        'format-unicode':['stdlib_format_cli_acceptance','char_scalar_cli_parity','char_operator_cli_parity'],
        'artifact-target':['artifact_target_contract_cli']}
    results=[]
    env=dict(LUNA_E1_RELEASE_CLI=str(RELEASE))
    for label,suites in batches.items():
        args=[CARGO,'test','--locked','-p','luna-cli']
        for s in suites:args+=['--test',s]
        results.append(run('focused/'+label,args,cwd=SRC/'luna-rs',required=False,extra_env=env))
    results.append(run('focused/registry',[CARGO,'test','-p','luna-common','registry_parity'],cwd=SRC/'luna-rs',required=False))
    results.append(run('focused/whole-file-io',[CARGO,'test','-p','luna-driver','--test','whole_file_io_v1_acceptance_tests'],cwd=SRC/'luna-rs',required=False))
    save(OUT/'focused-summary.json',dict(tested_commit=PIN,status='PASS' if all(r['exit']==0 for r in results) else 'FAIL',results=results))
    if not all(r['exit']==0 for r in results): raise RuntimeError('focused stage failed')

def diagnostics():
    """DIAG-1..10 structural audit at the exact pin: registry parity, per-code
    identity/span policy, producer inventory drift, and the span-anchor policy
    executed over the diagnostic fixture corpus."""
    results=[]
    results.append(run('diagnostics/registry-parity',[CARGO,'test','-p','luna-common','registry_parity'],cwd=SRC/'luna-rs',required=False))
    results.append(run('diagnostics/code-policy',[CARGO,'test','-p','luna-common','every_registered_code_declares_its_identity_and_span_policy'],cwd=SRC/'luna-rs',required=False))
    results.append(run('diagnostics/span-anchor',[CARGO,'test','-p','luna-common','source_diagnostics_are_anchored_or_declared_contract_diagnostics'],cwd=SRC/'luna-rs',required=False))
    results.append(run('diagnostics/inventory',['python',SRC/'docs/tools/diag_producer_inventory.py','--check'],required=False))
    fixtures=[]
    for path in sorted((SRC/'tests/luna/language').rglob('*.ln')):
        relative=path.relative_to(SRC).as_posix()
        fixtures.append(relative)
    save(OUT/'fixtures.json',dict(tested_commit=PIN,count=len(fixtures),fixtures=fixtures))
    save(OUT/'diagnostics-summary.json',dict(tested_commit=PIN,status='PASS' if all(r['exit']==0 for r in results) else 'FAIL',results=results))
    if not all(r['exit']==0 for r in results): raise RuntimeError('diagnostics stage failed')

def examples():
    run('docs-examples',['python',SRC/'docs/tools/check_v01_examples.py','--gate','--cli',CLI,'--runtime-lib',BUILD/'runtime/libluna-runtime.a','--out',OUT/'docs-examples'])
    run('docs-validator',['python',SRC/'docs/tools/validate_v01_docs.py'])

def quickstart():
    root=OUT/'quickstart';root.mkdir(parents=True,exist_ok=True)
    program=root/'program.ln'
    program.write_text('fn main() -> i32 {\n    return 0;\n}\n',encoding='utf-8',newline='\n')
    exe=root/'program.exe'
    run('quickstart-build',[CLI,'build',program,'-o',exe],cwd=root)
    run('quickstart-native',[exe],cwd=root)
    pe=exe.read_bytes()
    assert pe[:2]==b'MZ'
    obj=exe.with_suffix('.obj')
    save(OUT/'quickstart.json',dict(tested_commit=PIN,status='PASS',source_sha256=sha(program),
        executable_sha256=sha(exe),object_sha256=sha(obj) if obj.exists() else None,
        formats=dict(executable_pe64_x86_64=pe[:2]==b'MZ',object_coff64_x86_64=(obj.read_bytes()[:2]==b'\x64\x86') if obj.exists() else False),
        cli_release_sha256=sha(RELEASE),time=utc()))

def comptime():
    """Comptime/layout qualification using the R5 harness, fetched from the R5
    evidence branch (the harness is not part of the tested checkout) and pinned
    to the final implementation SHA."""
    harness=OUT/'harness-comptime'
    assert not harness.exists(), 'Fresh comptime evidence path already exists'
    shutil.copytree(Path(__file__).resolve().parent/'comptime', harness, ignore=shutil.ignore_patterns('__pycache__'))
    runner=harness/'run_qualification.py'
    run('comptime/qualification',['python',runner,'--checkout',SRC,'--sysroot',SRC/'luna-rs',
        '--debug',CLI,'--release',RELEASE,'--runtime',BUILD/'runtime/libluna-runtime.a','--label','final1'],cwd=harness)
    summary=harness/'final1'/'summary.json'
    if summary.exists():
        save(OUT/'comptime-summary.json',json.loads(summary.read_text(encoding='utf-8')))

def workspace():
    env=dict(LUNA_E1_RELEASE_CLI=str(RELEASE))
    triage=run('workspace-triage',[CARGO,'test','--workspace','--no-fail-fast'],cwd=SRC/'luna-rs',required=False,extra_env=env)
    final=run('workspace-final',[CARGO,'test','--workspace'],cwd=SRC/'luna-rs',required=False,extra_env=env)
    save(OUT/'workspace-summary.json',dict(tested_commit=PIN,triage=triage,final=final,status='PASS' if triage['exit']==final['exit']==0 else 'FAIL'))
    if triage['exit'] or final['exit']: raise RuntimeError('workspace stage failed')

def workspace2():
    """Re-run of the exact-pin workspace final pass after the host session ended
    mid-run. The interrupted attempt stays preserved under its own label; the
    completed triage result is reused as immutable evidence."""
    env=dict(LUNA_E1_RELEASE_CLI=str(RELEASE))
    triage=json.loads((OUT/'workspace-triage.json').read_text(encoding='utf-8'))
    final=run('workspace-final2',[CARGO,'test','--workspace'],cwd=SRC/'luna-rs',required=False,extra_env=env)
    save(OUT/'workspace-summary2.json',dict(tested_commit=PIN,
        note='workspace-triage reused from the interrupted-run evidence; workspace-final2 is a fresh exact-pin run after the host session ended mid-run',
        triage=triage,final=final,status='PASS' if triage['exit']==final['exit']==0 else 'FAIL'))

if __name__=='__main__':
    OUT.mkdir(parents=True,exist_ok=True);TOP.mkdir(parents=True,exist_ok=True)
    parser=argparse.ArgumentParser()
    parser.add_argument('stage',choices=['acquire','build','focused','diagnostics','examples','quickstart','comptime','workspace','workspace2','all'])
    parser.add_argument('--resume-clone',action='store_true')
    args=parser.parse_args()
    try:
        if args.stage=='all':
            for stage in ('acquire','build','focused','diagnostics','examples','quickstart','comptime','workspace'):
                globals()[stage]()
                event('STAGE_COMPLETED',stage=stage)
        elif args.stage=='acquire':acquire(args.resume_clone)
        else:globals()[args.stage]()
    except BaseException as error:
        event('STAGE_FAILED',stage=args.stage,error=repr(error));raise
    event('STAGE_COMPLETED',stage=args.stage)
