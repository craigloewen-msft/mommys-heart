# Replace CRM terminology with accurate app terminology

The repository currently presents Mommy's Heart as a CRM in user-facing copy,
documentation, code comments, placeholder URLs, and technical artifact names. The
application instead supports case management, evidence, messaging, clients, and
volunteers, so the CRM label is unnecessarily narrow and misleading.

## Terminology decision

Use the existing **Mommy's Heart** brand without a category suffix in
user-facing text. In descriptive prose, use **app**, **application**, or the
specific feature being discussed. For technical artifacts that need a stable
machine name, replace `mommys-heart-crm` / `mommys_heart_crm` with
`mommys-heart-app` / `mommys_heart_app`. Replace the fictional
`crm.example.org` host with `app.example.org`.

Do not introduce a different broad category such as “case management system” as
a universal product label; use specific descriptions only where they add useful
context.

## Changes

- Change the browser title from “Mommy's Heart CRM” to “Mommy's Heart”.
- Rewrite notification-email guidance such as “Sign in to the Mommy's Heart CRM”
  as natural brand-only wording, including both HTML and plain-text variants.
- Update README, environment guidance, migration comments, module docs, log
  messages, and other code comments to say app/application or name the relevant
  feature directly.
- Rename the Cargo package and Leptos output to `mommys-heart-app`, update the
  Rust crate imports and tracing target, and regenerate the lockfile.
- Keep the Leptos stylesheet reference, release binary paths, runtime output
  setting, CLI documentation, and container command synchronized with the new
  artifact name.
- Update dev-container display text to use the brand without the old category.
- Change all `crm.example.org` fallback/sample URLs to `app.example.org` without
  otherwise altering URL fallback behavior.

## Acceptance criteria

- No case-insensitive `CRM` references or old `mommys-heart-crm` /
  `mommys_heart_crm` identifiers remain in application source, configuration,
  migrations, build files, or maintained documentation. Historical task records
  may retain the term as context.
- Browser and email copy identify the product simply as “Mommy's Heart”.
- Cargo, Leptos, telemetry, and container references all agree on
  `mommys-heart-app`; generated assets are requested under their actual names.
- Existing routes, data model, environment variable names, and behavior remain
  unchanged.

## Verification

- Run `cargo fmt --check`.
- Run `etc/dev.sh -- cargo check --no-default-features --features ssr`.
- Run `etc/dev.sh build` to exercise both SSR and hydrated Leptos outputs, then
  confirm the generated CSS/JS artifact names match the updated app references.
- Start the app with `etc/dev.sh run`, wait for `MH_READY`, and confirm the page
  title and stylesheet load without browser console or network errors.
- Render the email preview gallery and inspect/search the output to confirm both
  HTML and plain-text wording use only the Mommy's Heart brand.
- Search the repository outside historical task files for remaining old terms,
  including case variants, old crate identifiers, and the old placeholder host.
- Validate the Containerfile references the renamed release binary and Leptos
  output consistently; build it when the container runtime is available.
