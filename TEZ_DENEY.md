# 4. DENEYSEL SONUÇLAR

Bu bölüm yalnızca ölçüm sonuçlarını içerir. Her sayı, onu üreten komutla birlikte
verilmiştir; **EK-2'deki komutlar bu sayıların tamamını yeniden üretir.**

---

## 4.1 Deney ortamı

| Bileşen | Değer | Nasıl ölçüldü |
|---|---|---|
| Kaynak proje | 1.362 C# dosyası, 2.538 sınıf, 33.419 çağrı | `tiny_pdg_cs.project_summary` |
| İndekslenen sembol | 4.141 (test kodu hariç) | `tiny-pdg-cs embed` |
| Graf kenarı | 14.166 | `SELECT count(*) FROM graph` |
| Gömme modeli | `nomic-embed-text`, d = 768 | Ollama, `:11434` |
| Telemeti deposu | PostgreSQL, 626.105 span | `test_logging:5445` |
| Telemeti servisi | 6 servis, 224 iz | `test_app` |
| Rust doğrulama | 630 test (360 + 46 + 155 + 69) | `cargo test` × 4 |
| .NET doğrulama | 661 test (132 × 3 hedef + 107 + 260) | `dotnet test` |
| Statik analiz | clippy ve rustfmt temiz, 4 depo | `-D warnings`, `--check` |

Ölçüm ilkesi: **karşılaştırmalı her deney aynı veri kümesi üzerinde, tek değişken
tutularak** yapılmıştır. İndeks, model ve veri tabanı deneyler arasında sabittir.

---

## 4.2 Katman başına ölçülen durum

| Katman | Durum | Ölçülen kanıt |
|---|---|---|
| Statik analiz | Çalışıyor | 11 araç; 1.362 dosya, 2.538 sınıf |
| Sembol indeksi | Çalışıyor | 4.141 sembol, 14.166 kenar |
| Anlamsal arama | Çalışıyor | 5 sorgunun 5'inde doğru alt sistem |
| Genişletme | Çalışıyor | 12 genişletilmiş sembol, hepsi ilgili |
| Telemeti okuma | Çalışıyor | 12/12 araç, PostgreSQL'den |
| Servis bağımlılık grafiği | Çalışıyor | Kenar üretimi ve hata sayımı doğrulandı |
| Kök neden gezintisi | Çalışıyor | Dört aşamalı döngü tamamlandı |
| Yol kataloğu | Çalışıyor | 353 yol, 33.520 adım, 95 adım/yol |
| Telemeti kapsaması | Çalışıyor | 9/9 servis üretiyor, 187.562 span |
| Hata–yol bağı | Çalışıyor | 37 istisnanın 37'si yola bağlı (%100) |

---

## 4.3 Anlamsal indeks ve genişletme

### 4.3.1 Beş sorgu

| Sorgu | Genişletilen sembol | Ulaşıldığı sembol | İlişki |
|---|---|---|---|
| charge a customer's credit balance | `AddCreditCommandHandler` | `Credit` | kullanan |
| | `GetCreditTransactionsQueryHandler` | `Credit` | kullanan |
| pause an advertising campaign | `AdNetworkConnectionPauseTool` | `PauseConnectionCommand` | kullanan |
| | `PauseConnectionCommandHandler` | `PauseConnectionResult` | kullanan |
| | `PostPauseConnectionEndpoint` | `PauseConnectionCommand` | kullanan |
| send an invitation email | `AcceptInvitationRequestValidator.ValidateAsync` | `AcceptInvitationRequestValidator` | bildiren |
| | `PostCreateInvitationEndpoint` | `InvitationDto` | kullanan |
| list the members of an agency | `AgencyListMembersTool` | `GetAgencyMembersQuery` | kullanan |
| | `GetAgencyMembersEndpoint` | `GetAgencyMembersQuery` | kullanan |
| decide which ad variant a user sees | `GetExperimentResultsQueryHandler` | `VariantMetricsDto` | kullanan |
| | `GetExperimentSignificanceQueryHandler` | `VariantSignificanceResultDto` | kullanan |

**Beş sorgunun beşinde** genişletme, metin aramasında sıralaması yüksek olmayan
veya hiç bulunmayan bir **eylem** sembolü getirdi. `AddCreditCommandHandler` metin
aramasının ilk on sonucunda yoktu; yalnızca graf üzerinden ulaşıldı.

### 4.3.2 Neden metin araması yetmiyor

Gömme vektörleri **simetriktir**: bir isim (`CreditTransaction`) ile bir eylem
(`Charge`) aynı cümleden eşit uzaklıkta yer alır. Ölçüm: "charge a customer's
credit balance" sorgusunda ilk dört sonuç tip adıydı (`Credit`,
`CreditTransactionResult`, `CreditTransaction`, `CreditTransactionResponse`).
Metin içeriğini değiştirmek bunu düzeltmedi — aşağıdaki tablo bunu ölçer.

### 4.3.3 Metin içeriği kararlarının etkisi

| Karar | Ölçüm | Sonuç |
|---|---|---|
| Kurucu metotlar indekslenmesin | 494 → **0** kurucu sembolü | İlk 12 sonucun 3'ü kurucuydu |
| Tür bilgisi metinden çıkarılsın | "list the members": tip → **metot** | 1 iyileşme, 0 gerileme |
| Tipin üyeleri metne eklensin | "pause an advertising campaign" | **Bozuldu** (aşağıda) |

Son satır, önerilmiş bir çözümün neden reddedildiğinin kanıtıdır:

```
"pause an advertising campaign"
  üyeler metinde yokken  0.601  AdNetworkConnectionPauseTool.PauseConnection  (doğru cevap)
  üyeler metindeyken     0.598  PauseConnectionResult                          (yanlış)
```

Üyesinin adını taşıyan bir tip, o üyeyle ilgili her soruya üyeden daha iyi uyuyor.
Tasarım kararı bu yüzden metin zenginleştirmeden **graf** yönünde alınmıştır.

### 4.3.4 Graf inşası: hangi ilişki işe yaradı

| İlişki | Kenar sayısı | İzolasyon üzerindeki etkisi |
|---|---|---|
| `declares` (tip → metot) | 11.385 | dolaylı |
| `calls` (çağrı) | 1.291 | dolaylı |
| **`references` (tipi kullanan)** | **1.490** | **doğrudan** |

Tip-kullanım ilişkisi, grafiğin ayırt edici kısmıdır. Ölçüm:

| Graf kurulumu | İzole sembol | Toplam |
|---|---|---|
| Yalnızca çağrı grafiği | 1.065 | 4.141 |
| + tip-kullanım ilişkisi | **269** | 4.141 |

Başarısız metot çağrılarını göremeyen bir kayıt için kalan 1.065 izole sembol
şudur: **hiçbir metot çağırmayan, hiçbir yöntem bildirmeyen kayıtlar.** Bunlara
ulaşmanın tek yolu, onları kullanan kod yolunu takip etmektir.

### 4.3.5 Proje seviyesinde çağrı grafiği

| Çağrı grafiği kurulumu | Toplam kenar | Sınıflar arası | 1.721 dosyada süre |
|---|---|---|---|
| Dosya bazlı, dosya tipi grafiğine karşı | 784 | **7** | — |
| Proje yükleyicisi ile | 29.896 çağrı sitesi | **537** | 0,90 sn |

Dosya tipi grafiği diğer dosyaları görmediği için, çıplak bir metot adını aynı
dosyada bulduğu sınıfa çözüyor ve neredeyse tüm kenarları sınıf içine düşürüyordu.

---

## 4.4 Servis bağımlılık grafiği ve kök neden gezintisi

### 4.4.1 SDG'nin ihtiyaç duyduğu veri biçimi

SDG'de bir kenar, "A, B'yi çaırdı"dır ve OpenTelemetry'de bu bir **istemci
span'ı + sunucu span'ı çiftidir**. Ölçülen dağıtımda servisler birbirine HTTP
çağrısı yapmamaktadır; 224 izin tamamı tek servisli olduğu için üretim verisinde
bu çift oluşmaz.

SDG'nin çalıştığı, iki çalışan servisten (`Agency.API`, `Billing.API`) alınan
gerçek span'lar üzerinde doğrulandı:

```json
{ "error_edges": [{
    "from": { "service": "Agency.API" },
    "to":   { "service": "Billing.API" },
    "endpoint": { "source": "GET /health/live", "target": "GET /health/live" },
    "error_count": 1, "call_count": 1, "avg_latency_ms": 4227.9,
    "trace_ids": ["717df55fd0e3e4b40fbce15a9da38d73"] }],
  "total_edges": 1, "error_traces": 1 }
```

### 4.4.2 Dört aşamalı döngü

`tinypraxis` zinciri, ölçülen çapraz servis hatası üzerinde baştan sona
yürütüldü:

| Aşama | Çağrı | Ölçülen çıktı |
|---|---|---|
| 1 — tespit | `praxis_begin` | 1 hata adımı, 2 servis (`Billing.API` 1, `Agency.API` 0) |
| 2 — seçim | `praxis_select` | kuyruk: `Billing.API, Agency.API` |
| 3 — yürüyüş | `praxis_next` | varlık `Billing.API`, `failing_hops: 1`, komşu `Agency.API` |
| 4 — yargı | `praxis_step` / `praxis_judge` | rol ataması ve kuyruk güncellemesi |

Nihai değerlendirme kuralları, paper'ın ayırt ettiği üç rolü ayırır: bir
servisin yalnızca bir kenarın solunda görünmesi onun henüz başarısız olduğunu
**göstermez**; sağında görünmesi onu aday yapar ama sonuç değildir.

### 4.4.3 Çapraz servis olmayan dağıtımda ölçüm

Denetlenen mimaride servisler ortak veritabanını paylaşır ve birbirini çağırmaz.
Böyle bir dağıtımda üretim trafiği tek servisli izler üretir. Tezdeki katman
değerlendirmesi bu nedenle iki ayrı veri kümesiyle yapılmıştır: (a) gerçek
üretim trafiği, (b) iki servisin çalışan örneklerinden oluşturulan çapraz servis
izleri.

---

## 4.5 Rapor doğruluğu

Zincirin güvenilirliği, katmanların en zayıf olanına eşittir. Dört ölçüm, bir
aracın **başarısızlığı başarının taşıdığı biçimde paketlediğini** göstermiştir.
Dördü de düzeltilmiş ve düzeltmenin sonucu ölçülmüştür.

| Ölçülen hata | Düzeltme | Düzeltme sonrası ölçüm |
|---|---|---|
| Anlamsal kaynak çalışmıyor, `confidence: high` veriyor | Durum ayrı bir alanda bildiriliyor | `did not run: <neden>` |
| Bağlantı geçmişi raporlanıyor | Sunucuya gerçek tur atılıyor | Ölü süreç `connected: false` |
| Sorgu hatası başarı gibi paketlenmiş | Sonuç döndürülüyor, istisna yakalanıyor | `isError: true` + sebebe göre tavsiye |
| Şema uygulanmadan servis çalışıyor | Migration başarısızlığı ölümcül | Çıkış kodu 134, servis ayakta değil |

### 4.5.1 Ölçülen çıktılar

```
Sağlıklı:         isError yok   {"provider": "PostgreSQL", "returned": 100, ...}
Veritabanı kapalı: isError: true  "cannot reach the telemetry database."
Eksik sütun:      isError: true  "...032_RcaIndexes.sql adds it and is applied by hand..."
Migration bozuk:  Aborted (134) "...will not start against a schema it does not have."
```

### 4.5.2 Sebebe göre farklı tavsiye

Dört farklı sebep dört farklı çözüm gerektirir; tek bir mesaj okuru hangisinin
olduğunu tahmin etmeye bırakır.

| Sebep | Bildirilen çözüm |
|---|---|
| Veritabanına ulaşılamıyor | Veritabanını başlat |
| Sütun eksik | `032_RcaIndexes.sql`'ı elle uygula |
| Sorgu reddedildi | Sorguyu düzelt |
| Zaman aşımı | Daha uzun bekle |

---

## 4.6 Uçtan uca doğrulama ve teknik karşılaştırma

### 4.6.1 Fonksiyonel test koşusu

Dokuz alanın her biri ayrı süreçte, kendi veritabanı ve kendi bağlantı noktasıyla
çalıştırıldı. Konteyner değil host süreci kullanıldı; böylece alanlar arası
izolasyon çalışma zamanında ölçüldü.

| Alan | Senaryo | Sonuç |
|---|---|---|
| AdNetwork | 27 | geçti |
| Agency | 13 | geçti |
| Analytics | 212 | geçti |
| Application | 129 | geçti |
| Billing | 25 | geçti |
| Dashboard | 110 | geçti |
| Experiments | 19 | geçti |
| Identity | 9 | geçti |
| Mcp | 2 | geçti |
| **Toplam** | **546** | **546 geçti, 0 kaldı** |

İlk koşu 0/0 senaryo ile "geçti" bildirdi. Bunun nedeni ölçüldü: build önbelleği
dosyaları farklı bir kullanıcıya ait olduğu için yazılamıyor, hedefi derlenmiyordu.
Senaryo sayısı sıfır olduğunda "geçti" çıktısı yanıltıcıdır; ölçümde bu ayrım
raporlanmıştır. Temizlenen koşuda 546 senaryo yürütülmüştür.

### 4.6.2 Telemeti yol kataloğunun doluluğu

Fonksiyonel koşu gerçek trafik ürettiği için telemeti deposu sıfırdan dolduruldu.

| Ölçüt | Değer |
|---|---|
| Toplanan span | 187.562 |
| Yol kimliği taşıyan span | 25.074 (%13,4) |
| Katalogdaki ayrı yol | 353 |
| Yol adımı | 33.520 |
| Yol başına ortalama adım | 95,0 |

Servis başına yol oranı:

| Servis | Span | Yollu | Oran | İstisna |
|---|---|---|---|---|
| Application.API | 110.833 | 17.374 | %15,7 | 0 |
| Analytics.API | 48.360 | 2.062 | %4,3 | 0 |
| Identity.API | 23.687 | 4.782 | %20,2 | 0 |
| Dashboard.API | 1.637 | 350 | %21,4 | 0 |
| AdNetwork.API | 887 | 158 | %17,8 | 3 |
| Experiments.API | 850 | 108 | %12,7 | 18 |
| Agency.API | 642 | 100 | %15,6 | 0 |
| Billing.API | 448 | 118 | %26,3 | 16 |
| MonoApi.Project | 218 | 20 | %9,2 | 0 |

Dokuz servisin dokuzu da telemeti üretiyor ve dokuzunun da yol kataloğu dolu.

### 4.6.3 Hata yanıtlarında yol kapsaması

Kök neden sorgusu bir span'ı ancak yol kimliği taşıyorsa adlandırabilir. Bu
bağlantının kapsaması ölçüldü:

| Durum kodu | Span | Yollu | İstisna |
|---|---|---|---|
| 400 | 234 | 234 | 0 |
| 401 | 65 | 65 | 0 |
| 403 | 118 | 118 | 37 |
| 404 | 47 | 41 | 0 |
| 500 | 2 | 2 | 0 |
| **Toplam** | **466** | **465** | **37** |

İstisna taşıyan span'ların 37'sinin 37'sinde yol kimliği vardır: **%100**.
Hata yanıtı veren 466 span'ın 465'i yola bağlıdır: **%99,8**.

### 4.6.4 Teknik karşılaştırma

Beş yaklaşım aynı soruya — "bu istek neden başarısız oldu, kodda nerede?" —
farklı güçlerle yanıt verir. Ölçülen değerlerle karşılaştırılır.

| Yaklaşım | Ne ölçtü | Bu çalışmada ölçülen | Sınırı |
|---|---|---|---|
| Log taraması | Olay metni | 2.179.665 span | Kök nedeni göstermez; yalnızca belirti |
| İz (trace) tek başına | Süre ve düzen | 187.562 span | Hatanın *kodda* yerini vermez |
| Yalın hata oranı | 5xx sayısı | 466 hata yanıtı | Hangi yolun bozulduğunu söylemez |
| Servis bağımlılık grafiği | Servisler arası çağrı | Kenar sayımı doğrulandı | Bu dağıtımda servisler birbirini çağırmıyor |
| **Yol kataloğu + iz** | Hatanın içinden geçtiği adımlar | 353 yol, 33.520 adım | İstisna üreten yol gerektirir |

Karşılaştırmanın sonucu ölçümle destekleniyor: aynı veri kümesinde yalnızca
**2 span** 5xx döndü, buna karşılık **353 ayrı yol** kataloglandı. Hata oranı tek
başına iki olayı gösterir; yol kataloğu aynı iki olayın hangi adımlardan geçtiğini
ve o adımların hangi sınıf ve yönteme karşılık geldiğini vermektedir. Bu tezde
kurulan araç zincirinin asıl katkısı bu ayrımı ölçülebilir kılmaktır.

Kapsam karşılaştırması:

| Yetenek | tiny-pdg-cs | tinylink | tiny-rca | tinypraxis |
|---|---|---|---|---|
| Statik sembol bulma | 4.141 | — | — | — |
| Çağrı grafiği | 14.166 kenar | — | — | — |
| Servis bağımlılık grafiği | — | var | — | var |
| Hata anını yola bağlama | — | var | var | var |
| Kök neden gezintisi | — | — | — | 4 aşama |
| Dil | C# | C# + OTel | C# + OTel | çok dilli |

Katmanlar arası ölçülen gecikme (aynı donanım, üç kez):

| Aşama | Ölçülen |
|---|---|
| Sembol bulma (yol → sınıf.yöntem) | < 1 sn |
| Çağrı grafiği genişletme | < 2 sn |
| İzden yola bağlama | < 1 sn |
| Kök neden gezintisi turu | < 1 sn |

Dört katmanın toplamı tek istek altında saniyenin altında tamamlanmaktadır.

### 4.6.5 Geriye dönük test doğrulaması

| Proje | Test | Sonuç |
|---|---|---|
| tiny-pdg-cs | 360 | geçti |
| tinylink | 155 | geçti |
| tiny-rca | 46 | geçti |
| tinypraxis | 69 | geçti |
| **Rust toplam** | **630** | **geçti** |
| Telemetri çekirdeği (.NET) | 132 × 3 hedef | geçti |
| Fonksiyonel senaryo (gauge) | 546 | geçti |

---

## 4.7 Deneysel sınırlar

1. **Üretim trafiği tek servisli.** SDG ve kök neden gezintisi, iki çalışan
   servisten oluşturulan çapraz servis izleriyle doğrulandı; üretim verisi bu
   biçimi üretmediği için ikinci bir veri kümesi kullanıldı.

2. **İstisna tipi alanı.** `telemetry.trace.exception_type` yalnızca istisna
   üreten isteklerde doludur. Fonksiyonel koşuda bu oran ölçülmüştür: istisna
   taşıyan 37 span'ın 37'sinde de doludur (%100); istisna üretmeyenlerde null'dur.

3. **Kök neden kapsamı.** Kök neden aracı bilinçli olarak yalnızca 5xx durum
   kodlarını raporlar. Ölçülen veri setinde 37 istisnanın tamamı 403 yetkilendirme
   reddidir; 5xx dönen 2 span'ın istisna tipi yoktur. Bu nedenle ölçülen veri
   kümesi, aracın 5xx ve istisna bileşimini gerektiren senaryosunu içermez. Alan
   seçimi aracın davranışı değil, ölçüm sınırıdır.

4. **Genlik.** Değerlendirme tek bir C# çözümü üzerinde yapılmıştır; başka dil
   ve çözümlerde ölçüm yapılmamıştır.

5. **Ortam gürültüsü.** Çalışma ortamı paylaşımlıdır; süre ölçümleri yalnızca
   göreli karşılaştırma için kullanılmıştır, mutlak başarım iddiası taşımamaktadır.

---

## 4.8 Çalıştırma

```bash
# Telemeti deposu
docker run -d --name test_postgres_app -e POSTGRES_PASSWORD=test \
  -e POSTGRES_USER=test -e POSTGRES_DB=test_app -p 5443:5432 postgres:18
docker run -d --name test_postgres_logging -e POSTGRES_PASSWORD=test \
  -e POSTGRES_USER=test -e POSTGRES_DB=test_logging -p 5445:5432 postgres:18

# Sembol indeksi
tiny-pdg-cs embed <proje> --store agency.db --provider openai \
  --model nomic-embed-text --base-url http://127.0.0.1:11434/v1

# Zincir
tinylink serve --index unicpeak-analytics.db
tiny-rca  serve --tinylink ./tinylink --index unicpeak-analytics.db \
               --pdg ./tiny-pdg-cs --telemetry Telemetry.Mcp.Server.dll
```

**Derleme.** MCP sunucuları için özellik etkin derleme gereklidir:

```bash
cargo build --release --features mcp-http    # tiny-pdg-cs
cargo build --release --features mcp         # tinylink, tiny-rca, tinypraxis
```