# Security Policy

## Supported Versions

AikenFlow is currently a v0.1 technical preview. Security fixes target the
latest `main` branch and the latest tagged release once releases begin.

## Reporting a Vulnerability

Report suspected vulnerabilities privately to the repository owner or through a
GitHub security advisory if enabled for the repository.

Do not report protocol-level findings from generated audit output as compiler
vulnerabilities unless AikenFlow claimed stronger assurance than it produced.
In v0.1, AikenFlow is not a formal verification tool.

## Assurance Boundary

AikenFlow review artefacts are derived from explicit protocol IR. Optional
repository-intelligence helpers such as `ast-outline`, agent context files, and
protocol draft reports are navigation aids only and are not trusted semantics.
