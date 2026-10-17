"""Standalone R5-3 recorder; only run during parent-authorized CLI window.

python run_qualification.py --checkout FRESH_CHECKOUT --sysroot FRESH_BUILT_SYSROOT
    --debug DEBUG_CLI --release RELEASE_CLI --runtime RUNTIME_A

All generated roots, native artifacts and logs stay beside this script.
No Cargo use, no canonical sysroot mutation, no checkout edits.
"""
import argparse, datetime, hashlib, json, os, pathlib, re, shutil, subprocess, time

P = argparse.ArgumentParser()
for name in ("checkout", "sysroot", "debug", "release", "runtime"):
    P.add_argument("--" + name, required=True, type=pathlib.Path)
P.add_argument("--label", default="acquisition1")
P.add_argument("--critical-only", action="store_true")
A = P.parse_args()
ROOT = pathlib.Path(__file__).resolve().parent
PIN = "c085449b207b5860b1d7a79bee221c2ff3d357c3"
OUT = ROOT / A.label
OUT.mkdir(exist_ok=False)
plan = json.loads((ROOT / "plan.json").read_text(encoding="utf-8"))
assert subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=A.checkout, text=True).strip() == PIN
assert not subprocess.check_output(["git", "diff", "--name-only", "HEAD"], cwd=A.checkout, text=True).strip()
assert not subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=no"], cwd=A.checkout, text=True).strip()
for row in plan["rows"]:
    assert hashlib.sha256((ROOT/row["relative_path"]).read_bytes()).hexdigest() == row["sha256"], row["relative_path"]

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

events, verdicts = [], []
base_env = os.environ.copy()
base_env["LUNA_RUNTIME_LIB"] = str(A.runtime.resolve())
base_env["NO_COLOR"] = "1"
binary_provenance = {name:dict(path=str(path.resolve()),sha256=digest(path)) for name,path in [("debug",A.debug),("release",A.release),("runtime",A.runtime)]}
(OUT / "provenance.json").write_text(json.dumps(dict(tested_commit=PIN, checkout=str(A.checkout.resolve()), sysroot=str(A.sysroot.resolve()), binaries=binary_provenance,plan_sha256=digest(ROOT/"plan.json")),indent=2)+"\n",encoding="utf-8")

def record(label, command, root):
    event = dict(label=label, command=[str(x) for x in command], tested_commit=PIN,
                 cwd=str(OUT), sysroot=str(root), started_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
    env = base_env | {"LUNA_SYSROOT":str(root.resolve())}
    start=time.monotonic()
    try:
        result = subprocess.run(event["command"], cwd=OUT, env=env, capture_output=True, timeout=120)
        stdout,stderr=result.stdout,result.stderr
        event.update(exit=result.returncode, classification="completed")
    except subprocess.TimeoutExpired as error:
        stdout,stderr=error.stdout or b"",error.stderr or b""
        event.update(exit=None, classification="TIMEOUT_FAIL")
    for kind,data in [("stdout",stdout),("stderr",stderr)]:
        path=OUT/f"{label}.{kind}.log"
        path.write_bytes(data)
        event[kind]=dict(path=str(path),sha256=hashlib.sha256(data).hexdigest())
    event["duration_seconds"]=round(time.monotonic()-start,3)
    (OUT/f"{label}.json").write_text(json.dumps(event,indent=2)+"\n",encoding="utf-8")
    events.append(event)
    return event,stderr.decode("utf-8",errors="replace")

def save_summary():
    (OUT/"summary.json").write_text(json.dumps(dict(tested_commit=PIN, gates=verdicts, pass_count=sum(x["result"]=="PASS" for x in verdicts), fail_count=sum(x["result"]=="FAIL" for x in verdicts), characterization_count=sum(x["result"]=="CHARACTERIZED" for x in verdicts), events=events),indent=2)+"\n",encoding="utf-8")

def verdict(label, ok, **details):
    verdicts.append(dict(label=label,result="PASS" if ok else "FAIL",**details))
    save_summary()
    print(json.dumps(verdicts[-1]),flush=True)

roots={}
for mode,extensions in [("source",(".ln",".toml")),("artifact",(".llib",".obj",".toml"))]:
    root=OUT/f"{mode}-root"
    src=A.sysroot/"libs"/"external"
    for path in src.rglob("*"):
        if path.is_file() and path.suffix in extensions:
            target=root/"libs"/"external"/path.relative_to(src)
            target.parent.mkdir(parents=True,exist_ok=True)
            shutil.copy2(path,target)
    manifest=[dict(path=str(p.relative_to(root)),sha256=digest(p)) for p in root.rglob("*") if p.is_file()]
    (OUT/f"{mode}-root-manifest.json").write_text(json.dumps(manifest,indent=2)+"\n",encoding="utf-8")
    assert not list(root.rglob("*.ln")) if mode=="artifact" else not list(root.rglob("*.llib"))
    roots[mode]=root

critical=["if_expression_false.ln","if_expression_true.ln","if_statement_control.ln","branch_function_values.ln","match_expression_values.ln"]
standalone=[ROOT/r["relative_path"] for r in plan["rows"] if "/dependency/" not in r["relative_path"] and pathlib.Path(r["relative_path"]).name!="enum_alignment_provider.ln"]
standalone.sort(key=lambda path:(0 if path.name in critical else 1,path.name))
if A.critical_only:
    standalone=[path for path in standalone if path.name in critical]

for profile,cli in [("debug",A.debug),("release",A.release)]:
    for mode,root in roots.items():
        projects=OUT/f"projects-{profile}-{mode}"
        projects.mkdir()
        for source in standalone:
            local=projects/source.name
            shutil.copy2(source,local)
        if not A.critical_only:
            provider=projects/"enum_alignment_provider.ln"
            shutil.copy2(ROOT/"inputs"/"layout"/provider.name,provider)
            if mode=="artifact":
                event,_=record(f"{profile}-{mode}-layout-provider",[cli,"build",provider,"--lib","--emit","llib","--quiet","-I",root,"-o",provider.with_suffix(".llib")],root)
                verdict(f"{profile}-{mode}-layout-provider",event["exit"]==0)
                if event["exit"]==0: provider.unlink()
        for source in standalone:
            local=projects/source.name
            label=f"{profile}-{mode}-{source.stem}"
            expected=next(r["expected"] for r in plan["rows"] if ROOT/r["relative_path"]==source)
            exe=projects/f"{source.stem}.exe"
            negative=expected.startswith("check_and_build_exit1")
            historical=expected.startswith("characterization_only")
            checks=[]
            if negative:
                check,stderr=record(label+"-check",[cli,"check",local,"--quiet","-I",root],root)
                checks.append(dict(command="check",exit=check["exit"],e4005=stderr.count("error[E4005]"),span=bool(re.search(r"--> .*\.ln:\d+:\d+",stderr) and "^" in stderr)))
            build,stderr=record(label+"-build",[cli,"build",local,"--quiet","-I",root,"-o",exe],root)
            if historical:
                observed=None
                if build["exit"]==0 and exe.exists():
                    run,_=record(label+"-run",[exe],root)
                    observed=run["exit"]
                verdicts.append(dict(label=label,result="CHARACTERIZED",expected=expected,build_exit=build["exit"],observed_exit=observed,claim="historical final-if statement spelling has no block tail expression; this is not a value-producing if conformance claim"))
                save_summary()
                print(json.dumps(verdicts[-1]),flush=True)
            elif negative:
                checks.append(dict(command="build",exit=build["exit"],e4005=stderr.count("error[E4005]"),span=bool(re.search(r"--> .*\.ln:\d+:\d+",stderr) and "^" in stderr)))
                verdict(label,all(c["exit"]==1 and c["e4005"]==1 and c["span"] for c in checks) and not exe.exists(),expected=expected,observations=checks,executable_exists=exe.exists())
            elif build["exit"]==0 and exe.exists():
                run,_=record(label+"-run",[exe],root)
                verdict(label,run["exit"]==0,expected=expected,build_exit=build["exit"],run_exit=run["exit"])
            else:
                verdict(label,False,expected=expected,build_exit=build["exit"],executable_exists=exe.exists())

if not A.critical_only:
    for profile,cli in [("debug",A.debug),("release",A.release)]:
        for scenario in ["source_baseline","taken_replacement","untaken_replacement"]:
            mode="source" if scenario=="source_baseline" else "artifact"
            root=roots[mode]
            project=OUT/f"dependency-{profile}-{scenario}"
            shutil.copytree(ROOT/"inputs"/"dependency",project)
            label=f"dependency-{profile}-{scenario}"
            publish_ok=True
            if mode=="artifact":
                for name in ["r5_taken","r5_untaken","r5_branch"]:
                    source=project/f"{name}.ln"
                    event,_=record(label+f"-publish-{name}",[cli,"build",source,"--lib","--emit","llib","--quiet","-I",root,"-o",source.with_suffix(".llib")],root)
                    verdict(label+f"-publish-{name}",event["exit"]==0)
                    if event["exit"]!=0:
                        publish_ok=False
                        break
                    source.unlink()
            if not publish_ok:
                continue
            consumer=project/"consumer.ln"
            exe=project/"baseline.exe"
            build,_=record(label+"-baseline-build",[cli,"build",consumer,"--quiet","-I",root,"-o",exe],root)
            baseline_ok=False
            if build["exit"]==0 and exe.exists():
                run,_=record(label+"-baseline-run",[exe],root)
                baseline_ok=run["exit"]==0
            verdict(label+"-baseline",baseline_ok,build_exit=build["exit"])
            if mode=="source" or not baseline_ok:
                continue
            assert not any((project/f"{name}.ln").exists() for name in ["r5_taken","r5_untaken","r5_branch"])
            (project/"baseline-bundles.json").write_text(json.dumps([dict(path=str(path),sha256=digest(path)) for path in project.iterdir() if path.suffix in [".llib",".obj"]],indent=2)+"\n",encoding="utf-8")
            changed="r5_taken" if scenario=="taken_replacement" else "r5_untaken"
            original=(ROOT/"inputs"/"dependency"/f"{changed}.ln").read_text(encoding="utf-8")
            old,new=("return 23;","return 29;") if changed=="r5_taken" else ("return 11;","return 17;")
            assert old in original
            source=project/f"{changed}.ln"
            source.write_text(original.replace(old,new),encoding="utf-8")
            event,_=record(label+"-replacement-publish",[cli,"build",source,"--lib","--emit","llib","--quiet","-I",root,"-o",source.with_suffix(".llib")],root)
            verdict(label+"-replacement-publish",event["exit"]==0)
            if event["exit"]!=0:
                continue
            source.unlink()
            (project/"replacement-bundles.json").write_text(json.dumps([dict(path=str(path),sha256=digest(path)) for path in project.iterdir() if path.suffix in [".llib",".obj"]],indent=2)+"\n",encoding="utf-8")
            check,checkerr=record(label+"-changed-check",[cli,"check",consumer,"--quiet","-I",root],root)
            changed_exe=project/"changed.exe"
            build,stderr=record(label+"-changed-build",[cli,"build",consumer,"--quiet","-I",root,"-o",changed_exe],root)
            stale=lambda event,text:event["exit"]==1 and "error[E6001]" in text and "dependency execution fingerprint mismatch" in text
            rejected=stale(check,checkerr) and stale(build,stderr) and not changed_exe.exists()
            if scenario=="taken_replacement":
                verdict(label+"-stale-rejection",rejected,check_exit=check["exit"],build_exit=build["exit"],executable_exists=changed_exe.exists(),expected="executed dependency body change MUST invalidate the materialized dependent")
            elif rejected:
                verdict(label+"-precision",True,classification="SOUND_STATIC_OVER_APPROXIMATION_C4_FU1",check_exit=check["exit"],build_exit=build["exit"])
            elif check["exit"]==0 and build["exit"]==0 and changed_exe.exists():
                run,_=record(label+"-changed-run",[changed_exe],root)
                verdict(label+"-precision",run["exit"]==0,classification="UNTaken_PROVIDER_NOT_INVALIDATING",run_exit=run["exit"])
            else:
                verdict(label+"-precision",False,classification="UNEXPECTED_REJECTION_OR_NATIVE_FAILURE",check_exit=check["exit"],build_exit=build["exit"])

save_summary()
print(str(OUT/"summary.json"))
raise SystemExit(1 if any(v["result"]=="FAIL" for v in verdicts) else 0)
