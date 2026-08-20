# Localization

## Scope

Fetch 0.1.5 localizes the embedded Vue interface in:

- English (`en`);
- Russian (`ru`);
- Tajik (`tg`, Cyrillic script).

English is always the first-run language and the fallback for an invalid saved
value or a missing catalog entry. Fetch deliberately does not infer a language
from the browser or operating system. The user chooses a language under General
settings and the interface changes without a reload.

The preference is stored in browser-local storage under `fetch.locale`, just
like the appearance preference. Each desktop or mobile client can therefore
choose independently without introducing server-side profiles, accounts, or a
database migration. The selected locale also updates the root HTML `lang`
attribute for assistive technology.

## Translation boundary

Translate application-owned interface text:

- navigation, headings, descriptions, tabs, fields, buttons, and tooltips;
- dialogs, confirmations, empty states, validation feedback, and errors created
  by the frontend;
- application status/stage labels, dates, times, counts, and progress summaries;
- screen-reader and other accessibility labels.

Do not translate user or external data:

- media and playlist titles;
- URLs, filesystem paths, runtime versions, and proxy endpoints;
- yt-dlp/FFmpeg output, retained logs, or server-provided diagnostic messages.

Dynamic external values remain visible verbatim inside translated surrounding
text. All three initial locales are left-to-right.

## Catalog and testing rules

English defines the canonical key set. Russian and Tajik catalogs must contain
the same leaf keys, and automated tests reject missing or extra entries. Use
named interpolation parameters for dynamic values and localization formatters
for application-owned dates, times, and numbers. Do not build sentences by
concatenating translated fragments.

Browser tests cover the English default and representative Russian/Tajik flows
on desktop and mobile. Layout checks must account for translated labels being
longer than their English equivalents.
