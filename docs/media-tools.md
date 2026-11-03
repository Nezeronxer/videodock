# MediaTools

`videodock_core::processing` обрабатывает локальные файлы. Аудио пользователя остаётся на машине. Rust выполняет скачивание и проверку моделей, управление заданиями, inference через проверенный C API и WPE. FFmpeg используется для локального декодирования, кодирования и копирования видеопотока.

```rust
pub fn available(data_dir: &Path) -> bool;
pub async fn process(
    input: PathBuf,
    effect: &str,
    output_dir: PathBuf,
    data_dir: PathBuf,
    cancel: CancellationToken,
    progress: Arc<dyn Fn(f64) + Send + Sync>,
) -> anyhow::Result<PathBuf>;
```

`progress` передаёт **проценты 0..100**. `available` проверяет поддержку платформы и допустимость папки без сетевых запросов и установки. `process` при первом вызове устанавливает необходимые модели автоматически.

| Эффект | Алгоритм | Результат |
|---|---|---|
| `extract_vocals` | Spleeter 2 stems, ONNX fp16 | WAV float32, 44100 Hz, stereo |
| `remove_vocals` | Spleeter accompaniment | Для аудио WAV; для видео MKV с исходным видеопотоком и AAC |
| `extract_voice` | DPDFNet2 speech enhancement | WAV float32, 16000 Hz, mono |
| `remove_echo` | Поканальная delayed WPE, 16 taps, 3 iterations | Для аудио WAV 48000 Hz stereo; для видео MKV с исходным видеопотоком и AAC |

WPE уменьшает поздние комнатные отражения. Это открытый алгоритм слепой дереверберации; тождество закрытой модели исходного приложения не заявляется. Результат зависит от сигнала и акустики.

Модели и CPU runtime находятся только в `data_dir/models`, вне Dropbox. Скачивание имеет предел размера и SHA256 pin; установка с неподтверждённым digest отклоняется. Архив runtime распаковывается после проверки SHA256, с проверкой путей, типов записей и размера. Ссылки и специальные файлы запрещены. Перед загрузкой C API байты установленных библиотек сверяются с проверенным архивом. Модели также повторно проверяются перед использованием.

CPU runtime закреплён на **sherpa-onnx 2.0.1**. Подготовлены Linux x86_64, Windows x86_64, macOS x86_64 и arm64. Реальный прогон выполнен на Linux; запуск на macOS/Windows требует отдельной проверки. Для системных FFmpeg/FFprobe используется общий `platform::media_tool`, включая путь внутри Windows пакета и Homebrew fallback для Finder.

Spleeter получает полный FFT-window контекст 4096 samples с обеих сторон: у закреплённого runtime `center=false`, и без контекста возникают выбросы ISTFT на границах. Для DPDFNet2 закреплённый runtime уже компенсирует задержку сдвигом `2*window_length`; его модель имеет `window_length=320`. Дополнительный хвост 640 samples сохраняет окончание сигнала, повторный сдвиг не применяется. Каждый результат сохраняет исходное количество декодированных кадров PCM. Смещение аудиодорожки относительно видео переносится при сборке MKV.

Обработка выполняется вне UI-потока. Один процесс приложения одновременно исполняет одно задание MediaTools, чтобы batch не создавал неограниченное число ONNX sessions. Обрабатываются блоки по 8 секунд с контекстом до 0,5 секунды по краям. Выполняющийся native вызов завершается перед освобождением моделей; отмена проверяется между вызовами. Задержка отмены зависит от CPU и текущего этапа. При отмене FFmpeg принудительно завершается и дожидается reap; приватная временная папка удаляется после завершения worker. Существующие пользовательские файлы не заменяются.

Декодированный PCM ограничен 2 GiB. Для профиля stems это примерно 101 минута, для WPE 93 минуты, для speech 9,3 часа. Превышение возвращает ошибку, готовый усечённый результат не публикуется. Проверяется доступное место для временных данных и результата.

Полные лицензии встроены в `crates/core/src/processing/notices` и сохраняются рядом с моделями вместе с `THIRD_PARTY.md`, `source-separation-pins.json` и `speech-pins.json`. Spleeter upstream имеет MIT-лицензию; отдельная лицензия в карточке конвертированных моделей не объявлена, что отмечено в notice. Sherpa/DPDFNet имеют Apache-2.0, ONNX Runtime и NARA-WPE - MIT.

Проверки:

```sh
CARGO_BUILD_JOBS=2 cargo test -p videodock-core --lib --test processing --offline
CARGO_BUILD_JOBS=2 cargo test -p videodock-core --test processing_real --test processing_video --offline -- --ignored --test-threads=1
```

Второй прогон использует официальные demo WAV, verified модели и установленные FFmpeg/FFprobe. Для отдельного холодного cache задаётся `VIDEODOCK_PROCESSING_TEST_DATA` на папку вне Dropbox.

Первичные источники: [sherpa C API v2.0.1](https://github.com/k2-fsa/sherpa-onnx/blob/v2.0.1/sherpa-onnx/c-api/c-api.h), [Spleeter runtime](https://github.com/k2-fsa/sherpa-onnx/blob/v2.0.1/sherpa-onnx/csrc/offline-source-separation-spleeter-impl.h), [DPDFNet runtime](https://github.com/k2-fsa/sherpa-onnx/blob/v2.0.1/sherpa-onnx/csrc/offline-speech-denoiser-dpdfnet-impl.h), [официальная публикация конвертированных моделей](https://huggingface.co/csukuangfj/sherpa-onnx-spleeter-2stems-fp16/tree/93ba771920ade509f8cbd6825b1a90856c797e08), [NARA-WPE](https://github.com/fgnt/nara_wpe).
