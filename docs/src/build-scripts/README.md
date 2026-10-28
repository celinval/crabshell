# What build scripts do

This chapter maps observed build-script behavior onto candidate sandbox
categories: named bundles of permissions (see
[Named grants](../design/capability-bundles.md)) that a crate can
request and a consumer can grant.

- [Observed labels](labels.md): the source data and how each label maps to a
  category.
- [Proposed categories](categories.md): the categories ordered by privilege,
  and how many crates each tier covers.
- [Observations](observations.md): what the data implies for the policy.
- [Category experiments](experiments.md): real crates of three categories
  built under the sandbox, and what they needed.
