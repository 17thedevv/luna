<!-- luna-doc-role: guidance -->

> **Luna 0.1 — guidance.** Current guidance. The versioned baseline and adopted amendments govern; implementation failures remain gaps, not semantic overrides. See the [versioned specification](../spec/0.1/README.md).

# Luna 0.1 documentation site

Static bilingual entry pages and generated HTML mirrors of docs/spec/0.1.
The versioned Markdown is authoritative; generated pages must not be edited as
an independent specification. Existing page URLs are preserved.

Regenerate from repository root:

```powershell
python docs/tools/build_v01_docs.py
python docs/tools/validate_v01_docs.py
python -m http.server 8000
```

Serve the repository root so links to adopted contracts/source files resolve.
Open /docs/site/vi/index.html or /docs/site/en/index.html. The language contract
is retained despite incomplete implementation; every page identifies release
conformance as blocked/unverified. Website publication is not part of this task.
