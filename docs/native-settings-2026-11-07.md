# Нативные модели настроек 4K Video Downloader+, 07.11.2026

Объект - тот же Windows x86-64 EXE с SHA-256 `50e3ea9303549d1700d2908af6025f8c2ce5512c8f5a0682ccc1dedbde2cebbc`. Разбор offline, только чтение: PE sections, Qt meta-object revision12, таблицы dispatch, jump thunks, машинные инструкции getter и ограниченные диапазоны дизассемблера LLVM. Оригинал не выполнялся; Ghidra не запускалась.

## Проверенная цепочка свойств

В установленном заголовке Qt `QMetaObject::Call` значение1 соответствует ReadProperty. Таблицы ReadProperty оригинала дают следующие ветви. Адреса виртуальные с image base `0x140000000`; ASLR runtime адреса не измерялись.

| Класс / свойство | Локальный индекс | Ветка ReadProperty | Thunk | Getter | Читаемое поле |
|---|---:|---|---|---|---|
| VideoDownloader::ViewModel::Settings / threadsListModel | 0 | 0x14163f8cb | 0x140129126 | 0x1415f1050 | this+0x50 |
| VideoDownloader::ViewModel::Settings / speedLimitsListModel | 1 | 0x14163f8de | 0x1401629e4 | 0x1415f0ff0 | this+0x58 |
| ViewModel::Settings / subscriptionUpdatePeriodModel | 5 | 0x140470c11 | 0x1400feb6f | 0x140453470 | this+0xc8 |

Это не имена функций, угаданные по строкам: совпали имя и индекс свойства в Qt metadata, адрес ветви в таблице, её call, переход thunk и фактическое чтение поля getter. Возвращаемое поле - указатель модели; конкретная память экземпляра и её runtime содержимое не наблюдались.

## Параллельные загрузки

Метод updateLists имеет локальный method index31. Native dispatch `0x14163ef80` ведёт через ветвь `0x14163f3b5` и thunk `0x1400680ac` к реализации `0x1415f1c90`. Та заполняет модель this+0x50 результатом построителя `0x1415f1060`, вызывая его через `0x14008e211`.

Границы построителя по PE .pdata: `[0x1415f1060, 0x1415f19b3)`. Наблюдаемые строки подписей: 1, 2, 4, 6, 8; Safe, Safe, Stable, Optimal, Risky. Они совпадают с текущими подписями SettingsDialog.qml. Это не доказывает, что каждое число равняется числу одновременно активных сетевых запросов: scheduler оригинала отдельно не исследован.

## Скорость

Нативная функция `[0x1415ec0c0, 0x1415ee723)` по .pdata строит модель this+0x58. В ней найдено11 подписей: Unlimited, 50 Mbps, 25 Mbps, 10 Mbps, 4 Mbps, 2 Mbps, 1 Mbps, 500 kbps, 250 kbps, 100 kbps, 50 kbps. Упаковка элементов проходит по возрастанию адресов стека от rbp+0x860 до rbp+0x9c0 с шагом0x20, затем список передаётся модели this+0x58.

VideoDock содержит тот же набор подписей, но после Unlimited располагает их от меньшей скорости к большей. Статически обнаружено отличие порядка создаваемых элементов. Финальный порядок отображения оригинала требует отдельной проверки возможной сортировки модели и живого меню. Единицы хранения и фактическое ограничение полосы из этих подписей не следуют; собственный перевод Mbps в байты/с нельзя приписывать оригиналу без отдельной трассировки.

## Интервал подписок

В `[0x140453480, 0x140453f61)` найдены шесть элементов с подписями Auto, Never, 7 days, 1 day, 6 hours, 1 hour. Исходный QML использует serverSubscriptionUpdatePeriod для подсказки Auto; его конкретное серверное значение и обновление в этой работе не установлены.

В VideoDock сейчас предлагаются Never, 5 min, 15 min, 30 min, 1 hour, 3 hours, 6 hours, 12 hours, 1 day. Таким образом, Auto и7days отсутствуют в меню, а часть собственных значений не подтверждена этим нативным списком. Это конкретный открытый пробел соответствия; подмена Auto произвольными24часами не выполнялась. Обработчик VideoDock уже допускает7days в текстовом протоколе, но это не равно наличию пункта интерфейса.

## Воспроизведение и границы

Локальные материалы находятся в `~/.local/share/videodock/research/mac-continuation-2026-11-06/`:

- inspect_settings_models.py проверяет полный digest EXE, PE headers, имена/индексы трёх свойств, таблицы ReadProperty, thunk/getter bytes и пять числовых подписей; все assert прошли.
- native-settings-verified-final.json фиксирует машинно проверяемую часть, включая dispatch cases.
- shared-settings-metacall-body.asm, subscription-settings-metacall-body.asm, shared-settings-updatelists-body.asm, speed-presets-builder-body.asm и subscription-presets-fragment.asm сохраняют ограниченные диапазоны LLVM disassembly. Fragment включает код за границей функции; выводы выше относятся только к указанным диапазонам .pdata.

Эти локальные данные дополняют REA inventory Evidence `ev_762d93de83e8a2b4c943f3a5d5db2f48c99cd0e7a754d955b3cd083a683c2e0a`. Для собственных чтений и LLVM не выдумываются REA Evidence IDs. Открытый REA unknown `unk_b06b66fa17bae95bc1c60a12342cdbe111ac1e6990ec4c8537ea6006fb40facd` частично уточнён адресами и подписями; семантика моделей и Auto остаётся открытой.
