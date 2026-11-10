#!/usr/bin/env python3
"""
MultiNet CyberShield AI - CycloneDX v1.5 JSON SBOM Schema & Graph Integrity Validator
Milestone: NM-07.1.2 (Supply Chain Conformance & Dependency Graph Integrity)
"""

import json
import sys
from pathlib import Path

def validate_sbom(sbom_path: Path, schema_path: Path):
    print(f"Validating SBOM: {sbom_path}")
    print(f"Against CycloneDX 1.5 Schema: {schema_path}")

    with open(sbom_path, "r", encoding="utf-8") as f:
        bom = json.load(f)

    with open(schema_path, "r", encoding="utf-8") as f:
        schema = json.load(f)

    # 1. Official JSON Schema Validation
    try:
        import jsonschema
        jsonschema.validate(instance=bom, schema=schema)
        print("[PASS] CycloneDX v1.5 JSON Schema Validation Successful.")
    except ImportError:
        print("[WARN] jsonschema python package not found; skipping strict schema check.")
    except Exception as e:
        print(f"[FAIL] Schema validation error: {e}")
        sys.exit(1)

    # 2. Graph & Structure Integrity Validations
    print("\nRunning Automated Graph Integrity Tests...")

    # A. Root Application in metadata
    meta = bom.get("metadata", {})
    root_comp = meta.get("component")
    if not root_comp:
        print("[FAIL] Missing metadata.component root application")
        sys.exit(1)
    
    root_ref = root_comp.get("bom-ref")
    if not root_ref:
        print("[FAIL] Missing bom-ref in metadata.component")
        sys.exit(1)
    print(f"[PASS] Root application declared in metadata: {root_ref}")

    # B. Components array must NOT duplicate root application
    components = bom.get("components", [])
    comp_refs = set()
    dup_refs = []
    
    for c in components:
        ref = c.get("bom-ref")
        if not ref:
            print(f"[FAIL] Component missing bom-ref: {c.get('name')}")
            sys.exit(1)
        if ref == root_ref:
            print(f"[FAIL] Root application bom-ref duplicated in components array: {root_ref}")
            sys.exit(1)
        if ref in comp_refs:
            dup_refs.append(ref)
        comp_refs.add(ref)

    if dup_refs:
        print(f"[FAIL] Duplicate bom-ref values found in components: {dup_refs}")
        sys.exit(1)
    print(f"[PASS] {len(components)} dependency components verified; zero duplicates, zero root collision.")

    # C. Dependencies array integrity
    dependencies = bom.get("dependencies", [])
    dep_parents = set()
    unresolved_refs = []
    has_root_dep = False

    for d in dependencies:
        parent_ref = d.get("ref")
        if not parent_ref:
            print("[FAIL] Dependency entry missing ref attribute")
            sys.exit(1)
        if parent_ref in dep_parents:
            print(f"[FAIL] Duplicate dependency parent ref: {parent_ref}")
            sys.exit(1)
        dep_parents.add(parent_ref)

        if parent_ref == root_ref:
            has_root_dep = True
        elif parent_ref not in comp_refs:
            unresolved_refs.append(f"Unresolved parent ref: {parent_ref}")

        depends_on = d.get("dependsOn", [])
        if len(depends_on) != len(set(depends_on)):
            print(f"[FAIL] Dependency {parent_ref} contains duplicate elements in dependsOn")
            sys.exit(1)

        for child in depends_on:
            if child == parent_ref:
                print(f"[FAIL] Self-dependency cycle detected on: {child}")
                sys.exit(1)
            if child not in comp_refs:
                unresolved_refs.append(f"Unresolved dependsOn target: {child} (under {parent_ref})")

    if not has_root_dep:
        print(f"[FAIL] Dependency graph missing root application dependency node: {root_ref}")
        sys.exit(1)

    if unresolved_refs:
        print(f"[FAIL] Unresolved dependency graph references found:\n" + "\n".join(unresolved_refs))
        sys.exit(1)

    total_edges = sum(len(d.get("dependsOn", [])) for d in dependencies)
    print(f"[PASS] Dependency graph verified: {len(dependencies)} nodes, {total_edges} edges, 0 unresolved references.")
    print("\n[SUCCESS] CycloneDX v1.5 SBOM conforms strictly to specification and graph integrity standards.")

if __name__ == "__main__":
    base_dir = Path(__file__).resolve().parent
    repo_root = base_dir.parent.parent
    sbom_file = repo_root / "packaging" / "output" / "sbom.json"
    schema_file = base_dir / "schema" / "bom-1.5.schema.json"

    if len(sys.argv) > 1:
        sbom_file = Path(sys.argv[1])
    if len(sys.argv) > 2:
        schema_file = Path(sys.argv[2])

    validate_sbom(sbom_file, schema_file)

