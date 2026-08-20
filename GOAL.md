# Active Goal

Develop Fetch 0.1.5 as a complete multilingual frontend release:

- use English as the deterministic first-run and fallback language;
- support explicit English, Russian, and Tajik interface selections;
- persist the selected language locally per browser/device without adding
  accounts, server profiles, or a database migration;
- apply language changes immediately without reloading Fetch;
- translate all application-owned navigation, views, dialogs, settings,
  validation feedback, empty states, actions, and accessibility labels;
- localize application-owned dates, times, counts, and status labels while
  leaving media metadata, URLs, filesystem values, retained logs, and external
  process messages unchanged;
- keep desktop and mobile layouts usable with longer translated labels;
- update the document language and use English whenever a translation key or
  saved locale is unavailable;
- cover defaulting, persistence, switching, translation completeness, and
  desktop/mobile flows with automated tests and synchronized documentation.

Do not add automatic browser-language detection, server-side user preferences,
machine translation, user accounts, or regress the mandatory architecture.
