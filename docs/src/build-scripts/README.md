# What build scripts do

This chapter studies what real build scripts need, to decide what the
defaults and [named grants](../policy/grants.md) should cover. The
goal is a light spec: most scripts should run with little to no annotations, while
defaults stay narrow enough to confine scripts and keep cache keys precise
(see the [design guidelines](../design.md#guidelines)).

It maps observed build-script behavior onto candidate categories: named
bundles of permissions that a crate can request and a consumer can grant.

- [Observed labels](labels.md): the source data and how each label maps to a
  category.
- [Proposed categories](categories.md): the categories ordered by privilege,
  and how many crates each tier covers.
- [Observations](observations.md): what the data implies for the policy.
- [Category experiments](experiments.md): real crates of three categories
  built under the sandbox, and what they needed.
