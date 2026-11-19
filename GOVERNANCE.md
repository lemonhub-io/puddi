# Puddi governance

> **Status: draft, not in effect.**
>
> Puddi's community is still small, so this model has not been adopted. It is written down now so that contributors know where the project is heading and so that it can be switched on without a rushed debate later. Until it takes effect, the [interim rules](#interim-rules) at the end of this document apply. Nothing described in the other sections, such as teams, votes or the RFC repository, exists yet.

## Interim rules

These are the rules today.

1. The project is run by its maintainers: the members of the [puddi-lang](https://github.com/puddi-lang) GitHub organization with write access to the repositories.
2. Decisions are made in public, in issues and pull requests. Maintainers decide when there is no agreement.
3. Significant changes (language behavior, standard library APIs, toolchain commands, the manifest format) are discussed in an issue before a pull request.
4. The specification changes only together with conformance cases for the change, as described in [CONTRIBUTING.md](CONTRIBUTING.md).
5. The [code of conduct](CODE_OF_CONDUCT.md) applies, and the maintainers enforce it.

## When this model takes effect

The maintainers adopt this model, possibly amended, by opening a pull request that removes the draft notice, once **any two** of these hold:

- at least five people outside the founding maintainers have each had at least five pull requests merged in the last twelve months;
- at least three active maintainers, not all affiliated with the same employer;
- other projects depend on Puddi in production, so that breaking changes need a predictable process;
- the volume of language proposals is more than the maintainers can discuss informally.

The adopting pull request stays open for at least 14 days and needs approval from a majority of the maintainers.

## Principles

- **The specification is the source of truth.** The compiler, the standard library and the documentation follow [SPEC.md](SPEC.md). Behavior that is not in the specification is not a promise.
- **Executable agreement.** Every normative statement in the specification is backed by conformance cases. A proposal is not done until its cases are.
- **Small and coherent beats large and clever.** A feature is added when it fits the language's existing ideas and pays for its complexity.
- **Open by default.** Decisions, their reasons and their dissent are recorded in public.

## Roles

### Contributors

Anyone who files an issue, reviews, writes code, documentation or conformance cases, or helps others. No permission is needed.

### Reviewers

Contributors trusted to review and approve pull requests in one or more areas (for example the type checker, code generation, the standard library, the documentation). A reviewer's approval counts towards merging in their area.

A maintainer nominates a reviewer after sustained, high-quality contributions in an area. The nomination passes by lazy consensus of the maintainers over seven days.

### Maintainers

Maintainers have write access to the repositories, merge pull requests, cut releases, take part in votes and enforce the code of conduct. They are collectively responsible for the project's direction.

A maintainer nominates a new maintainer, usually from the reviewers. The nomination passes with a two-thirds majority of the maintainers. Maintainers who are inactive for twelve months move to emeritus status and can return by asking. A maintainer can be removed for a code of conduct violation, or for acting against the project's interest, by a two-thirds majority of the other maintainers.

### Teams

When there are enough maintainers, areas with distinct concerns may get their own teams, each with its own reviewers:

| Team | Responsible for |
|---|---|
| Language | SPEC.md, the RFC process, the conformance suite |
| Compiler and toolchain | the `pud` command, code generation, the runtime |
| Library | the prelude and the `std` modules |
| Documentation | puddi-docs and the documentation in each repository |

A team decides matters in its area. Matters that span teams go to the maintainers as a whole.

The list of maintainers, reviewers and teams will live in a `MAINTAINERS.md` file once this model is adopted.

## Decision making

### Lazy consensus

Most decisions are made by lazy consensus: a proposal (a pull request, an issue or a message) states what will happen. If no maintainer objects within the review period, it is accepted. Silence is consent.

| Decision | Review period |
|---|---|
| Ordinary pull requests | none beyond the usual review |
| Nominating a reviewer | 7 days |
| Accepting an RFC (final comment period) | 10 days |

An objection must give reasons and, where possible, a way forward.

### Voting

When consensus cannot be reached, any maintainer can call a vote. Votes are held in a public issue and stay open for at least seven days. Each maintainer has one vote: +1, 0 or -1.

| Matter | Passes with |
|---|---|
| Ordinary decisions | a simple majority of the votes cast |
| Normative specification changes, adding or removing a maintainer, changing this document | two thirds of all maintainers |

Votes are rare. Needing one is a sign that the proposal should be reworked.

## Language changes: the RFC process

Normative changes to the specification, new standard library modules, and changes to the manifest or package format go through an RFC. Bug fixes, editorial changes to the specification, and additions that follow existing patterns (for example a missing method alongside its siblings) do not.

1. **Pre-RFC.** Optional discussion in an issue to see whether there is interest.
2. **RFC.** A pull request to a `puddi-rfcs` repository with a document using its template: motivation, the proposed specification text, examples, effect on existing programs, alternatives, and open questions.
3. **Discussion.** The RFC is revised in the open. A responsible team (or the maintainers) shepherds it.
4. **Final comment period.** When the discussion settles, a maintainer proposes to accept, postpone or reject it. The final comment period lasts ten days and is announced in the project's channels.
5. **Decision.** Without new, substantive objections the RFC is accepted (or postponed or rejected). Normative specification changes need the two-thirds threshold if a vote is called.
6. **Implementation.** The specification change is merged with conformance cases. Cases that the compiler does not pass yet are marked `xfail` and linked to a tracking issue.

An accepted RFC can be revised by a new RFC if implementation shows it was wrong.

## Releases and stability

- Puddi follows [Semantic Versioning](https://semver.org/). In 0.x, a minor release (0.2, 0.3) may make breaking language or toolchain changes; a patch release (0.1.1) only fixes bugs.
- Each release states which version of the specification it implements and lists its known non-conformance from the conformance suite.
- Breaking changes are listed in [CHANGELOG.md](CHANGELOG.md) with migration notes. Where possible, a feature is deprecated with `@deprecated` for one minor release before removal.
- Version 1.0 freezes the language for backward compatibility. Before it, the maintainers will decide on the post-1.0 policy (for example editions) through an RFC.
- Any maintainer can cut a release by following [docs/releasing.md](docs/releasing.md). A release that contains a normative change needs approval from a second maintainer.

## Security

Security reports are handled privately as described in [SECURITY.md](SECURITY.md). Once there is a security team, it is a small group of maintainers who receive reports, coordinate fixes and publish advisories.

## Code of conduct enforcement

The [code of conduct](CODE_OF_CONDUCT.md) is enforced by the maintainers, or by a moderation team of at least two people once one exists. Anyone involved in a report is excluded from handling it. Decisions are recorded privately; bans are announced only to the extent needed to enforce them.

## Changing this document

Changes to this document are made by pull request, stay open for at least 14 days, and need a two-thirds majority of all maintainers. While the document is a draft, the maintainers may edit it freely, as long as the draft notice stays in place.
