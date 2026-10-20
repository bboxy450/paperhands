# Embedded evidence viewer

The shared-dispatch, bounded MCP and offline presentation patterns were inspected
in the local source and published `ontology` 1.0.0 crate. Their source bytes match.
Ontology copyright 2026 Gonzih; its MIT license is retained as `ONTOLOGY-LICENSE`.
Paperhands keeps its own experiment engine and adds no Ontology runtime dependency.
Its research DAG also uses Ontology's inspected entity/relation provenance profile
as the vocabulary for exact objects, revision links and attributed declarations.

The following exact assets are retained from that crate and compiled into the
Paperhands binary. They load without CDNs, a network connection or runtime files.

- Pele 1.6.0, MIT, `pele.global.js`, 682,005 bytes.
  SHA-256 `b82d1bcd435302779a945e1e0f4a54ea56fb26e010ce3f2122f4216fcfc095d7`.
  Upstream: <https://registry.npmjs.org/pele/-/pele-1.6.0.tgz>.
  License: `PELE-LICENSE`.
- DOMPurify 3.4.17, Apache-2.0, `purify.min.js`, 29,828 bytes.
  SHA-256 `cd37898f9782adf54820c413191b5691803f0f137c799d6edcfd38523b4c2c31`.
  Upstream: <https://www.npmjs.com/package/dompurify/v/3.4.17>.
  License: `DOMPURIFY-LICENSE`.

Rendering has source, edge and SVG size bounds. Links and images are disabled;
SVG passes through the bundled sanitizer. Typed datasets remain usable by other
frontends and survive a client rendering error. Browser isolation and MCP Apps
support belong to the host; the external-command runner is not a sandbox.
