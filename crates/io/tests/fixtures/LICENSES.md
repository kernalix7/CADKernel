# Real-World STEP/IGES Corpus — License Manifest

This directory holds real-world STEP/IGES test fixtures used by the
`real-corpus` Cargo feature in `cadkernel-io`. The default test suite does NOT
build or run these — see `crates/io/tests/real_corpus.rs`.

Every file listed below ships with an explicit license. Every entry includes a
sha256 hash so that future drift (intentional re-export, accidental re-encode)
is detectable. New fixtures must be added with a paired entry here in the same
PR; the corpus test file enumerates fixtures by name so unaccounted files are
ignored.

Phase 1 sources (this manifest) are limited to two origins with unambiguous
Apache-2.0-compatible terms:

- **NIST Engineering Design Model Repository**
  <https://github.com/usnistgov/engineering-design-models>
  Works of US Federal Government employees — public domain in the United
  States per 17 U.S.C. §105. Repository `LICENSE.md` confirms this and grants
  permission to freely use, copy, modify, and distribute.
- **STEPcode `data/` directory**
  <https://github.com/stepcode/stepcode>
  3-Clause BSD License (per `COPYING`). NIST-originated portions additionally
  in the US public domain (see `INTENT.md`).

Audit gap codes (G1–G10) refer to the gap matrix in the Phase 1 audit report.

## STEP files

| Filename | Bytes | sha256 | Source | License | Schema | Description / Gap closed |
|---|---:|---|---|---|---|---|
| `step/nist_design2.step` | 51470 | `48923dfe12485e6d7246a9a98bfbae59d31c957fb643cb110bef612bb427a701` | NIST EDM `models/STEP/design2/design2.step` | Public domain (17 U.S.C. §105) | AP203 (`CONFIG_CONTROL_DESIGN`) | Mechanical part. Closes G1 (no AP203 fixture). |
| `step/nist_part.step` | 109278 | `71f00a36546f89b469dc0b9a6d720acd49077206727dae617cf917b631b58731` | NIST EDM `models/STEP/part/part.step` | Public domain | AP203 | Generic part export. Closes G1. |
| `step/nist_clevis21.stp` | 59320 | `4fff420338c430410ff289fef8e9b90c182aea18b7ae38817670262a846ee55e` | NIST EDM `models/STEP/STI/clevis21/clevis21.stp` | Public domain | AP203 | Clevis fastener. Closes G1. |
| `step/nist_doghouse.stp` | 58484 | `2f3792c8481de20dcae1aad2cdabdc044046b5ea45417ab54995b496bcfb8e58` | NIST EDM `models/STEP/STI/doghouse/doghouse.stp` | Public domain | AP203 | Bracket-style structural part. Closes G1. |
| `step/nist_as1_pe.stp` | 81925 | `5a46af594d0e3a192aad66fc27d21a8edae32c96c1a9b6185a143787e4874b65` | NIST EDM `models/STEP/STI/as1_pe/as1_pe.stp` | Public domain | AP203 | AS1 Pro/E export — small assembly. Closes G1, partial G3 (assembly structure). |
| `step/nist_bracket1.stp` | 76201 | `e17a7657b0f251e93713a201bd3dc393a9f905ae477e43952bd9170d17f1a7ff` | NIST EDM `models/STEP/STI/bracket1-Part/bracket1-Part.stp` | Public domain | AP203 | Bracket. Closes G1. |
| `step/nist_interacting_pockets.stp` | 66665 | `472b123bb6a1231821f0415239a87d6802eeb4f006332ec02202f3ef8dc43fbf` | NIST EDM `models/STEP/STI/interactingPockets/interactingPockets.stp` | Public domain | AP203 | Multi-pocket feature interaction model. Closes G1. |
| `step/stepcode_as1_oc_214.stp` | 441968 | `038be659c54b16c9f3da8d7b2da7b63e3fd8879d3abe5b7826108336a7c0bae9` | STEPcode `data/ap214e3/as1-oc-214.stp` | BSD-3-Clause | AP214e3 (`AUTOMOTIVE_DESIGN`) | AS1 demonstrator assembly via Open CASCADE export. Closes G3 (assembly product structure). |
| `step/stepcode_dm1_id_214.stp` | 87564 | `25e727cdc4d738064da7907d604bd4ceeb4e941c6fdc201f3df04c0b94671661` | STEPcode `data/ap214e3/dm1-id-214.stp` | BSD-3-Clause | AP214e3 | Drawing-management exchange — surface + B-Rep mix. Broadens AP214 coverage. |
| `step/stepcode_io1_cm_214.stp` | 41720 | `eb234e26bca411a6da39c311e74d5a47b5e7a1248ae589a3a86cd2dee1f91f9b` | STEPcode `data/ap214e3/io1-cm-214.stp` | BSD-3-Clause | AP214e3 | Pro/E export — configuration-managed part. Broadens AP214 coverage. |
| `step/stepcode_sg1_c5_214.stp` | 23827 | `227467b4584d3bd5ddf5c96906649cb2f0f62901fbfb00c3993a17e49155e539` | STEPcode `data/ap214e3/sg1-c5-214.stp` | BSD-3-Clause | AP214e3 | CATIA V5 STEP AP214 export — vendor-flavoured fixture. Closes partial G6 (real vendor exports). |

**STEP totals:** 11 files, 1098422 bytes (~1.05 MB). Schema coverage: AP203 (`CONFIG_CONTROL_DESIGN`) ×7, AP214e3 (`AUTOMOTIVE_DESIGN`) ×4. AP242 / PMI fixtures remain a gap (deferred to Phase 2 because the NIST CAD-PMI test models live on the `mbe.nist.gov` website and need per-file review before committing).

## IGES files

| Filename | Bytes | sha256 | Source | License | Origin | Description / Gap closed |
|---|---:|---|---|---|---|---|
| `iges/nist_d_part2.igs` | 12069 | `23dfc227baca0b619c0af69ad4583e98c428c3684a25bee077d99e144b6337ce` | NIST EDM `models/SDRC_I-DEAS/D_part2/D_part2.igs` | Public domain | I-DEAS | 56 IGES directory entries — surface part. Broadens IGES base coverage. |
| `iges/nist_d_bearing3.igs` | 23085 | `962607425b2ad1a27b65873518df6f1a3a06cd0bf8ee15784781b01cf0f10f76` | NIST EDM `models/SDRC_I-DEAS/D_bearing3/D_bearing3.igs` | Public domain | I-DEAS | Bearing geometry, 100 entities. Broadens IGES base coverage. |
| `iges/nist_flange_blank6.igs` | 30132 | `5d088a908a9ce0eaf893bdef63723b1b71c202747d9324cc4b9eb257239799f4` | NIST EDM `models/NIST/Manfred_Osti/suitcase/flange_blank6/flange_blank6.igs` | Public domain | Pro/E (different vendor) | Flange, 104 entities. Closes partial G6 (real vendor diversity). |
| `iges/nist_cadds_part_level1.igs` | 15633 | `881e592514578d27eb1a2bea1c46469cc427e9d520c2bd8d6f19dafaf70489b4` | NIST EDM `models/PTC/V18/des_ex/importing/cadds-part-level1.igs` | Public domain | Computervision CADDS (legacy) | Computervision IGES export — exercises legacy IGES dialect. Closes partial G6. |

**IGES totals:** 4 files, 80919 bytes (~79 KB). Two vendor flavours (SDRC I-DEAS, Pro/E, Computervision CADDS). Trimmed-surface, subfigure, and mixed-precision IGES files (G7, G8) remain Phase 2 gaps.

## Outstanding gaps after Phase 1

The following gaps from the audit are **not** closed by this phase and should
be addressed in Phase 2 (FreeCAD / CAx-IF / GrabCAD with per-file legal review):

- **G2** AP242 PMI / GD&T — needs NIST CAD-PMI Testing files (`mbe.nist.gov`)
  or CAx-IF round samples.
- **G4** Large assemblies (>100 parts) — tied to GPU-tessellation benchmarks.
- **G5** Trimmed NURBS surfaces with non-trivial UV trims.
- **G7** IGES Type 142/144 trimmed parametric surfaces.
- **G8** Mixed-precision IGES global-section quirks.
- **G9** Malformed-but-recoverable real corpora.
- **G10** STEP "complex entity" (AND-type) instances — Catia-flavoured exports.

## Verification

```bash
# default suite stays clean (corpus disabled)
cargo test --workspace

# corpus tier (15 fixtures: 11 STEP + 4 IGES, all expected to pass)
cargo test -p cadkernel-io --features real-corpus

# clippy at all-features must stay clean
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

To regenerate the sha256 column after adding a fixture:

```bash
cd crates/io/tests/fixtures
sha256sum step/*.stp step/*.step iges/*.igs
```
