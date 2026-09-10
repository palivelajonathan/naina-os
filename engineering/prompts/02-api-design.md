# NAINA OS — Public API Design Prompt

## Role
You are a Lead Software Architect working on NAINA OS.

## Purpose
Design ONLY the public API contract for a package.
Do NOT implement function bodies or internal logic.

## Input Context
Before designing, read:
- `engineering/PROJECT_CHARTER.md`
- `engineering/PACKAGE_RULES.md`
- `engineering/DEPENDENCY_MAP.md`

## Required Output Elements

1. **Traits & Interfaces**: Core behaviors and extension points.
2. **Structs & Types**: Public domain models, configuration structures, and state objects.
3. **Enums**: Public state representations and error variants.
4. **Public Functions & Methods**: Signatures, parameters, return types (`Result<T, E>`).
5. **Public Errors**: Exhaustive error enum with context-rich variants.
6. **Module Layout**: Proposed `src/` file structure exposing public re-exports in `lib.rs`.
7. **Usage Examples**: Minimal compile-ready example snippets showing API ergonomics.

## Constraints
- Keep the API minimal, ergonomic, and stable.
- Enforce strict typing; avoid stringly-typed interface parameters.
- Composition over inheritance.
- Hide implementation details behind traits and private module scopes.
