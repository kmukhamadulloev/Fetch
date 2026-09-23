export const processingMessages = {
  en: {
    title: 'Processes', subtitle: 'Downloads, conversions, edits, and metadata updates.', typeFilter: 'Process type',
    types: { all: 'All types', download: 'Downloads', conversion: 'Conversions', edit: 'Edits', metadata: 'Metadata updates' },
    states: { queued: 'Queued', running: 'Processing', completed: 'Completed', failed: 'Failed', cancelled: 'Cancelled', interrupted: 'Interrupted' },
    stages: { queued: 'Waiting for the media worker', processing: 'Processing media', publishing: 'Saving output', completed: 'Completed', failed: 'Failed', cancelled: 'Cancelled', interrupted: 'Interrupted' },
    elapsed: 'Elapsed {duration}', cancel: 'Cancel', retry: 'Retry from start', viewFile: 'View file', noJobs: 'No processes yet',
    noJobsHelp: 'Add a download or process a completed file.', unavailable: 'Processes are unavailable', actionFailed: 'The process action failed',
  },
  ru: {
    title: 'Процессы', subtitle: 'Загрузки, конвертация, правки и обновления метаданных.', typeFilter: 'Тип процесса',
    types: { all: 'Все типы', download: 'Загрузки', conversion: 'Конвертация', edit: 'Правки', metadata: 'Метаданные' },
    states: { queued: 'В очереди', running: 'Обработка', completed: 'Завершено', failed: 'Ошибка', cancelled: 'Отменено', interrupted: 'Прервано' },
    stages: { queued: 'Ожидание обработки', processing: 'Обработка медиа', publishing: 'Сохранение файла', completed: 'Завершено', failed: 'Ошибка', cancelled: 'Отменено', interrupted: 'Прервано' },
    elapsed: 'Прошло {duration}', cancel: 'Отменить', retry: 'Повторить с начала', viewFile: 'Показать файл', noJobs: 'Процессов пока нет',
    noJobsHelp: 'Добавьте загрузку или обработайте готовый файл.', unavailable: 'Процессы недоступны', actionFailed: 'Действие с процессом не выполнено',
  },
  tg: {
    title: 'Равандҳо', subtitle: 'Боргирӣ, табдил, таҳрир ва навсозии метамаълумот.', typeFilter: 'Навъи раванд',
    types: { all: 'Ҳамаи навъҳо', download: 'Боргириҳо', conversion: 'Табдилҳо', edit: 'Таҳрирҳо', metadata: 'Метамаълумот' },
    states: { queued: 'Дар навбат', running: 'Коркард', completed: 'Анҷом ёфт', failed: 'Хато', cancelled: 'Бекор шуд', interrupted: 'Қатъ шуд' },
    stages: { queued: 'Интизори коркард', processing: 'Коркарди медиа', publishing: 'Сабти файл', completed: 'Анҷом ёфт', failed: 'Хато', cancelled: 'Бекор шуд', interrupted: 'Қатъ шуд' },
    elapsed: 'Гузашт {duration}', cancel: 'Бекор кардан', retry: 'Аз аввал такрор кардан', viewFile: 'Нишон додани файл', noJobs: 'Ҳоло раванд нест',
    noJobsHelp: 'Боргирӣ илова кунед ё файли тайёрро коркард кунед.', unavailable: 'Равандҳо дастрас нестанд', actionFailed: 'Амали раванд иҷро нашуд',
  },
}
