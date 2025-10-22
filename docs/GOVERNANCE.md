# BitQuan Governance Framework

## Purpose
BitQuan governance ensures long-term sustainability, security, and openness of the network. This framework documents the roles, decision processes, and escalation paths required to keep the project community driven.

## Principles
- Transparent deliberation and published minutes for all material decisions
- Consensus-driven policy changes via the BitQuan Improvement Process (BQIP)
- Public, signed releases and verifiable build artifacts
- Conflict of interest disclosures for all maintainers and committee members

## Organizational Structure
### Lead Maintainer
- Coordinates overall technical direction and release readiness
- Acts as final escalation point for urgent security remediation
- Serves a renewable one-year term with community ratification

### Core Maintainers (≥3)
- Share responsibility for code review, merges, and roadmap execution
- Rotate primary on-call duties for security incidents and release windows
- Vote on policy changes and BQIP status transitions

### Steering Committee (5 members)
- Curates protocol-level roadmap and long-horizon research priorities
- Ratifies major consensus or cryptography changes (2/3 majority)
- Publishes quarterly strategy updates and risk assessments

### Community Council (9 members)
- Represents node operators, developers, auditors, and end users
- Runs public feedback channels and ensures documentation is accessible
- Facilitates elections and governance process reviews annually

## Decision Making
### Routine Changes
- Follow the standard GitHub pull request workflow with at least two maintainer approvals
- Proposals must reference relevant BQIP documents and include testing evidence

### Protocol Changes
- Require an accepted BQIP with implementation and threat analysis sections
- Steering Committee organizes public review windows (minimum 21 days)
- Activation follows published signaling criteria (e.g., miner vote, testnet stability)

### Emergency Actions
- Invoked only for critical security vulnerabilities or chain-halting bugs
- Emergency Response Team (Lead Maintainer + two Core Maintainers + one Steering member) acts unanimously
- Post-mortem published within 14 days including remediation steps

## Elections and Terms
- Elections occur annually using a verifiable voting mechanism (on-chain or audited off-chain)
- Staggered terms ensure continuity: half of each body renews every cycle
- Vacancies trigger a 30-day special election window

## Accountability and Transparency
- Meeting agendas, minutes, and voting records stored in `docs/governance/`
- Financial disclosures (if any treasury exists) published quarterly
- Conflict of interest statements updated whenever status changes

## Amendments
- Governance changes require a dedicated BQIP and 2/3 approval across Steering Committee and Community Council
- Amendments take effect no sooner than 30 days after ratification unless addressing a critical security issue
