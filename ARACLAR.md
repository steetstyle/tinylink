# ARAÇ KATALOĞU VE MATEMATİKSEL ÇALIŞMA

Beş sunucuda toplam **52 araç** bulunmaktadır. Her araç için önce hesapladığı
nicelik, sonra bunun biçimi verilmiştir.

| Sunucu | Araç sayısı | Katman |
|---|---|---|
| `tiny_pdg_cs` | 11 | Statik analiz ve semantik indeks |
| `tinylink` | 6 | İz–kod bağlama |
| `tiny_rca` | 10 | Kök neden analizi |
| `tinypraxis` | 13 | Kısıtlı graf gezintisi |
| `Telemetry.Mcp.Server` (.NET) | 12 | Telemeti sorgusu |
| **Toplam** | **52** | |

---

## 1. Ortak yapı

### 1.1 Gömme benzerliği

Sembol, sınıf ve yöntem adları bir metin gömme modeliyle vektöre dönüştürülür.
Model `nomic-embed-text`, boyut $d = 768$.

Verilen metinler $x, y$ için gömme $f: \mathcal{X} \rightarrow \mathbb{R}^{768}$:

$$\mathrm{emb}(x) = f(x), \qquad \mathrm{emb}(y) = f(y)$$

Kosinüs benzerliği:

$$\mathrm{sim}(x, y) = \frac{\mathrm{emb}(x) \cdot \mathrm{emb}(y)}{\lVert \mathrm{emb}(x) \rVert_2 \cdot \lVert \mathrm{emb}(y) \rVert_2}$$

Sorgu $q$ için sıralama, $S$ aday kümesi üzerinde:

$$\mathrm{rank}(q) = \big\{ s \in S \;\big|\; \mathrm{sim}(q, s) \ge \tau \big\}_{\text{azalan}}$$

Burada $\tau$ alt sınır, $k$ üst sınırdır. Dönen sonuç $R_q$:

$$R_q = \operatorname*{top-}k_{s \in S} \mathrm{sim}(q, s)$$

Ölçülen: beş sorgunun beşinde $\tau \ge 0{,}60$ eşiğiyle doğru alt sistem
bulunmuştur.

### 1.2 Güven ölçeği

İz ucunu koda bağlarken bir eşleşme güven puanı $c \in [0,1]$ taşır. Puan
eşleştirme stratejisine göre sabittir:

$$c(m) = \begin{cases}
0{,}95 & \text{hizmet içinde yol tablosu eşleşmesi} \\
0{,}85 & \text{gRPC ucu: "POST /service/Method" } \rightarrow \text{Sınıf.Metot} \\
0{,}70 & \text{çapraz servis eşleşmesi} \\
0{,}60 & \text{yolun son segmentinin metot adıyla anahtar eşleşmesi} \\
0{,}40 & \text{servis adı ile sınıf adının eşleşmesi} \\
0 & \text{eşleşme yok}
\end{cases}$$

Çapraz servis eşleşmesinin 0{,}70 ile sınırlanması ölçülmüştür: yol tablosu tüm
yüklenen projeleri birleştirdiği için kapsamsız bir en iyi eşleşme, %95'in en
yüksek güvenle karışabilirdi. Kapsam daraltıldığında çapraz eşleşme en fazla
0{,}70 alır.

Geri çağırmada kullanılan alt küme: $\{m \in M \mid c(m) \ge c_{\min}\}$.

### 1.3 Güçlendirilmiş seçim

En yüksek güvenli eşleşme tek başına seçilmez; tüm adaylar skorlanır.
Aday $m$ için:

$$\mathrm{score}(m) = c(m) \cdot \alpha + \beta \cdot \mathrm{coverage}(m)$$

burada $\mathrm{coverage}(m)$ eşleşen istek deseninin eşleşen kısmıdır.
En yüksek $\mathrm{score}$ taşıyan aday seçilir.

---

## 2. `tiny_pdg_cs` — 11 araç

### 2.1 Araç listesi

| # | Araç | Girdi | Çıktı |
|---|---|---|---|
| 1 | `project_summary` | yol | sınıf ve metot sayıları |
| 2 | `list_routes` | yol | HTTP rotaları |
| 3 | `find_callers` | `Sınıf.Metot` | ters kenarlar |
| 4 | `method_callees` | `Sınıf.Metot` | ileri kenarlar |
| 5 | `method_dependencies` | `Sınıf.Metot` | program bağımlılığı |
| 6 | `method_pdg` | `Sınıf.Metot` | temel bloklar ve kenarlar |
| 7 | `method_hammocks` | yol | yapısal çürüme |
| 8 | `find_context` | metin + mağaza | anlamsal arama |
| 9 | `find_patterns` | desen | metin deseni arama |
| 10 | `diff_impact` | iki sürüm | kenar kazancı/kaybı |
| 11 | `embed_index` | mağaza adı | indeksi doldurur |

### 2.2 Matematik

**Çağrı grafiği.** $G = (V, E)$ düğümleri metodları, kenarları çağrıları
temsil eder:

$$V = \{\, m \mid m \text{ bir metot}\,\}, \qquad E = \{\, (u, v) \mid u \text{ çağırır } v\,\}$$

Ölçülen: $|V| = 4\,141$, $|E| = 14\,166$.

**Geri çağırma.** `find_callers($t$)`:

$$\mathrm{Ancestors}(t) = \big\{ u \mid (u, v) \in E \text{ ve } v \in \mathrm{Ancestors}(t) \cup \{t\} \big\}$$

Genişletilmiş çağırma, kenarların `references` ilişkisiyle genişletilmiş hâlidir.
Genişletme sonrası yalıtılmış sembol sayısı $1\,065 \rightarrow 269$ azalmıştır.

**Gömme ile genişletme.** Sorgu $q$ için aday kümesi önce kosinüs ile
daraltılır, sonra çağrı grafiğinde genişletilir:

$$C(q) = \operatorname*{top-}k_{s \in S} \mathrm{sim}(q, s), \qquad \mathrm{Expand}(C) = C \cup N^+(C) \cup N^-(C)$$

**Program bağımlılığı grafiği (PDG).** $G_{pdg}$ üç kenarlıdır:

$$G_{pdg} = \big(V_{bb},\; E_{flow} \cup E_{ctrl} \cup E_{data}\big)$$

- $E_{flow}$: kontrol akışı — $b_i \leadsto b_j \iff j$, $i$ veya $i$'nin
  ardışık sürekliliğinden sonra ulaşılabilir
- $E_{ctrl}$: kontrol bağımlılığı — $b_i \leadsto b_j \iff j$, $i$'in bir
  koşulu üzerinde denetlenir
- $E_{data}$: veri bağımlılığı — $b_i \leadsto b_j \iff$ $j$ içinde
  tanımlanan bir değişken, $i$ içinde bir tanımı geçersiz kılar

**Yapısal çürüme (hammock).** Bir metodun çürüme oranı, gövdesinin
tutararsız olduğu blokların oranıdır:

$$\mathrm{decay}(m) = \frac{\#\{\, b \in V_{bb}(m) \mid \mathrm{inconsistent}(b) \,\}}{\#V_{bb}(m)}$$

**Fark etkisi.** İki sürüm $G_{-}$ ve $G_{+}$ arasında kazanılan ve kaybedilen
kenarlar:

$$\Delta E^{+} = E(G_{+}) \setminus E(G_{-}), \qquad \Delta E^{-} = E(G_{-}) \setminus E(G_{+})$$

Geri çağırma kaybı, değişiklikten etkilenen ve artık çağıranı olmayan
metotları verir:

$$L = \big\{ m \mid \mathrm{callers}(m) = \varnothing \text{ ve } m \in V(G_{+}) \setminus V(G_{-}) \,\}$$

---

## 3. `tinylink` — 6 araç

### 3.1 Araç listesi

| # | Araç | Girdi | Çıktı |
|---|---|---|---|
| 1 | `list_indexed_services` | — | servis ve metot sayıları |
| 2 | `find_methods` | desen | eşleşen metotlar |
| 3 | `blast_radius` | `Sınıf.Metot` | erişilen HTTP uçları |
| 4 | `explain_error` | OTel iz | metot, dosya, satır, güven |
| 5 | `ingest_logs` | OTLP yükü | yazılan kayıt sayısı |
| 6 | `search_logs` | metin + zaman | eşleşen kayıtlar |

### 3.2 Matematik

**Patlama yarıçapı.** Metot $m$'den erişilebilir uç kümesi, çağrı grafiğinde
sınırsız derinlikte ileri yönlü geçişle bulunur:

$$\mathrm{Br}(m) = \bigcup_{k \ge 0} N^{+k}(m) \;\cap\; \mathrm{Uç}$$

$\mathrm{Uç}$ HTTP ucu tanımlayan sınıflardır. $k$ derinlik sınırı verilirse
geçiş sonlandırılır. Çıktı ayrıca $m$'nin **çağıranlarını** ve **çağırdıklarını**
içerir, yani çift yönlüdür.

**İz–kod eşlemesi.** İzdeki hata sıçrayanı $h$, doğrudan handler ucuna
eşlenir. Doğrudan eşleşme bulunamazsa zincir yürütülür:

$$\mathrm{map}(h) = \underset{m \in M}{\arg\max}\; \mathrm{score}(m)$$

Her aday, §1.3'teki $\mathrm{score}$ ile değerlendirilir. Eşleşme, dönen
**güven puanı** ile birlikte verilir; puan $0$ ise eşleşme yoktur.

**Yol kimliği (`path_id`).** Bir istek boyunca kaydedilen enstrüman adımları
dizisi $P = \langle p_1, p_2, \dots, p_n \rangle$ alınır ve karma olarak
saklanır:

$$\mathrm{path\_id} = H\big(\mathrm{concat}(p_1 \Vert p_2 \Vert \dots \Vert p_n)\big)$$

Boş dizi için yol **yazılmaz**:

$$n = 0 \implies \mathrm{path\_id} = \texttt{NULL}$$

Bu koşul ölçülebilir bir sonuç doğurur: yalnızca enstrümanlanmış handler'dan
geçen istekler yol alır. Ölçülen kapsama:

| Servis | Span | Yollu | Oran |
|---|---|---|---|
| Application.API | 110.833 | 17.374 | %15,7 |
| Analytics.API | 48.360 | 2.062 | %4,3 |
| Identity.API | 23.687 | 4.782 | %20,2 |
| Dashboard.API | 1.637 | 350 | %21,4 |
| AdNetwork.API | 887 | 158 | %17,8 |
| Experiments.API | 850 | 108 | %12,7 |
| Agency.API | 642 | 100 | %15,6 |
| Billing.API | 448 | 118 | %26,3 |
| MonoApi.Project | 218 | 20 | %9,2 |

İstisna taşıyan 37 span'ın **37'sinde** yol kimliği vardır: %100.
Hata yanıtı veren 466 span'ın 465'i yola bağlıdır: %99,8.

---

## 4. `tiny_rca` — 10 araç

### 4.1 Araç listesi

| # | Araç | Girdi | Çıktı |
|---|---|---|---|
| 1 | `capabilities` | — | bağlı arka uç ve erişilebilirlik |
| 2 | `investigate_error` | OTel iz | adım → metot eşlemesi |
| 3 | `explain_error` | OTel iz | metot + güven |
| 4 | `explain_method` | `Sınıf.Metot` | program bağımlılığı |
| 5 | `method_dependencies` | `Sınıf.Metot` | bağımlılık dökümü |
| 6 | `method_hammocks` | yol | yapısal çürüme |
| 7 | `search_logs` | metin | eşleşen kayıt |
| 8 | `ingest_logs` | OTLP | yazılan kayıt |
| 9 | `telemetry_call` | istek | arka uca iletim |
| 10 | `diff_impact` / `search_github` | — | ileri arka uçlara geçiş |

### 4.2 Matematik

**Yetenek raporu.** `capabilities` kendi aleyhine bir `tools/list` turu
yapar ve dönen gerçek listeyi raporlar. Raporlanan alanlar:

$$\text{capabilities} = \big( C_{\text{bağlı}},\; C_{\text{kullanılamaz}},\; C_{\text{tur}} \big)$$

Kapatma testi gerçek turla yapılır; kapalı olmayan bir sunucu "kapalı" diye
raporlanmaz.

**Hata adımı eşlemesi.** İzdeki hata sıçrayanları $H = \{h_1, \dots, h_m\}$.
Her sıçrayan, §3.2'deki $\mathrm{map}$ ile bir metoda gider. Rapor, eşleşen
adımların kümesidir:

$$\mathrm{Match} = \big\{ (h, m, c) \;\big|\; c = c(\mathrm{map}(h)) > 0 \,\big\}$$$

Eşleşmeyen sıçrayanlar ayrıca listelenir; sessizce düşürülmez.

**Toplam güven.** Birden çok sıçrayan varsa, bir hata sıçrayanının güveni en
yüksek eşleşmesidir:

$$c(h) = \max_{m \in M} c(m \mid h)$$

---

## 5. `tinypraxis` — 13 araç

### 5.1 Araç listesi

| # | Araç | Girdi | Çıktı |
|---|---|---|---|
| 1 | `praxis_capabilities` | — | faz, eylem uzayı, bütçe |
| 2 | `praxis_begin` | olay | araştırma kimliği |
| 3 | `praxis_select` | aday kümesi | sıradaki varlık |
| 4 | `praxis_next` | — | sıradaki servis |
| 5 | `praxis_step` | varlık | bir bölge (tamamla veya atla) |
| 6 | `praxis_judge` | rol | varlığın rolü |
| 7 | `praxis_update_queue` | öneri | kuyruk güncellemesi |
| 8 | `praxis_report` | — | nihai rapor |
| 9 | `praxis_state` | — | anlık durum |
| 10 | `praxis_diff` | — | karşılaştırmalı etki |
| 11 | `praxis_explain` | metot | açıklama |
| 12 | `praxis_search` | metin | arama |
| 13 | `praxis_telemetry` | iz | telemeti bağlantısı |

### 5.2 Matematik

Bu katman bir **kısıtlı graf gezintisidir**. Gezinti bir kez daha erken
sonlanamaz; sonlanma kuyruk, bütçe ve kapalı eylem uzayı tarafından belirlenir.

**Tanımlar.** Servis bağımlılık grafiği $G_{sdg} = (S, E_{sdg})$; program
bağımlılığı grafiği $G_{pdg} = (V_{bb}, E_{pdg})$.

**Eylem uzayı (kapalı).** Her adımda dört eylemden biri seçilir:

$$\mathcal{A} = \{\, \textsf{Expand},\; \textsf{Relate},\; \textsf{Complete},\; \textsf{Discard} \,\}$$

- $\textsf{Expand}$: varlığın SDG komşularını kuyruğa ekler
- $\textsf{Relate}$: mevcut bulguyu varlığa bağlar
- $\textsf{Complete}$: bölge tamamlandı işaretler
- $\textsf{Discard}$: bölge değersiz işaretler

**Roller.** Her varlık **bir kez** yargılanır ve üç rolten birini alır:

$$\rho : S \rightarrow \{\, \textsf{PrimaryFailure},\; \textsf{SymptomOnly},\; \textsf{Unrelated} \,\}$$

**Kuyruk ve bütçe.** Kuyruk $Q$, karar verilmemiş varlıkları taşır:

$$Q_{t+1} = \big( Q_t \setminus \{\, v \,\} \big) \cup \mathrm{Dependencies}(v)$$

Kuyruk büyümesi yalnızca öneriye değil, **yargılanan servisin bağımlılıklarına**
bağlıdır; neden aşağı akışta ise grafiği izlemek tek yoldur.

Bütçe $B$ iki boyutludur:

$$B = (\, b_{\text{varlık}},\; b_{\text{blok}} \,), \qquad b_{\text{varlık}} = 2 \text{ veya } 5$$

**Gezinti kapanış koşulu.** Gezinti ancak

$$Q = \varnothing \;\wedge\; \forall v \in S:\; \rho(v) \neq \bot$$

sağlandığında kapanır. Kuyruk boşalana kadar rapor üretilemez; bu, raporun
karar değil sonuç olmasını sağlar.

**Dört aşamalı döngü.** Her varlık için:

$$\textsf{begin} \rightarrow \textsf{select} \rightarrow \textsf{next} \rightarrow \big[\textsf{step} \rightarrow \textsf{judge}\big]^{*} \rightarrow \textsf{report}$$

`select` atlanırsa kuyruk boş görünür. Ölçüm: bu hatanın kendisi bir ölçüm
hatası olarak yakalandı ve düzeltildi.

**Bölge ilerlemesi.** $\textsf{step}$ tek seferde bir bölge döndürür. Bölge
$\mathcal{B} = \{b_1, \dots, b_k\}$ ve

$$\textsf{Complete} \iff k \text{ bölgenin tamamı incelendi}, \qquad \textsf{Discard} \iff b \text{ ilgisiz}$$

**Dinamik SDG kenarı.** İki span aynı izde olsa bile kardeş ise kenar üretilmez.
Kenar ancak istemci–sunucu **çifti** oluşturduğunda üretilir:

$$\mathrm{parent}(c) = s \iff \mathrm{span\_id}(s) = \mathrm{parent\_id}(c)$$

Ölçülen: bu koşul sağlanmadığında kenarların tamamı boş kalır.

---

## 6. `Telemetry.Mcp.Server` — 12 araç

### 6.1 Araç listesi

| # | Araç | Girdi | Çıktı |
|---|---|---|---|
| 1 | `search_spans` | filtre | span listesi |
| 2 | `explain_trace` | `trace_id` | adım ağacı |
| 3 | `error_paths` | pencere | hatalı yollar |
| 4 | `path_profile` | kapsam | yol profili |
| 5 | `path_diff` | iki yol | ilk ayrışma |
| 6 | `new_paths` | pencere | yeni yollar |
| 7 | `trace_logs` | `trace_id` | iz günlükleri |
| 8 | `search_logs` | metin | eşleşen günlükler |
| 9 | `find_by_baggage` | anahtar/değer | taşıyan izler |
| 10 | `endpoint_health` | rota | yüzdelikler |
| 11 | `list_telemetry_scopes` | — | kapsam listesi |
| 12 | `capture_coverage` | — | yapılandırma ve tablo kapsamı |

### 6.2 Matematik

**Hatalı yollar.** Sorgu yalnızca sunucu tarafı, 5xx ve yol taşıyan span'ları
kapsar:

$$\mathrm{ErrPath} = \big\{ p \;\big|\; \mathrm{kind}(p) = \textsf{Server} \wedge \mathrm{status}(p) \in \{500,502,503,504\} \wedge \mathrm{path\_id}(p) \neq \texttt{NULL} \,\}$$

Yol kapsamı, span özniteliğinden değil katalogdan okunur:

$$\mathrm{scope}(p) = \mathrm{trace\_path}[\mathrm{path\_id}(p)].\mathrm{scope\_name}$$

Katalog boşlukları için birleştirme **sol dış** birleştirmedir; henüz yazılmamış
yola sahip bir hata, yıkama aralığının arkasında saklanmaz.

**İlk ayrışma.** İki yol $p, q$ adım dizileri üzerinde karşılaştırılır:

$$\mathrm{first\_diverge}(p, q) = \min \big\{ i \;\big|\; p_i \neq q_i \,\big\}$$

Böyle bir $i$ yoksa yollar aynıdır ve ayrışma bildirilmez.

**Yol sağlığı.** Bir kapsamın yolları trafik sırasına göre listelenir:

$$\mathrm{PathOrder}(s) = \mathrm{sort\_desc}\big(\{ p \mid \mathrm{scope}(p) = s \big\},\; \mathrm{last\_seen}(p)\big)$$

**Yüzdelikler.** Uç sağlığı, gecikmelerin sıralanmış dağılımından alınır:

$$p_{95} = \mathrm{quantile}_{0{,}95}\big(\{ \mathrm{latency}(r) \mid r \in R \,\}\big), \qquad p_{50} \le p_{95} \le p_{99}$$

---

## 7. Araçlar arası veri akışı

İstek bir hata verdiğinde zincir şu sırayla çalışır:

$$\underbrace{\mathrm{Telemetri}}_{\text{span + yol}} \rightarrow \underbrace{\mathrm{İz–kod}}_{\substack{c \in \{0.4,\dots,0.95\}}} \rightarrow \underbrace{\mathrm{PDG}}_{\text{bölge}} \rightarrow \underbrace{\mathrm{Gezinti}}_{\rho(v)}$$

Her aşamanın girdisi bir öncekinin çıktısıdır. Zincirin tek eksik olduğu
halka, istisna üreten ve enstrümanlanmış bir yolun bulunmamasıdır; bu koşulun
yol üzerindeki etkisi §3.2'de ölçülmüştür.

---

## 8. Ölçüm özeti

| Ölçüt | Değer |
|---|---|
| Araç sayısı | 52 |
| Sembol indeksi | 4.141 |
| Graf kenarı | 14.166 |
| Yol kataloğu | 353 yol / 33.520 adım |
| Toplanan span | 187.562 |
| İstisnalı span'ın yola bağlı olması | 37/37 (%100) |
| Hata yanıtının yola bağlı olması | 465/466 (%99,8) |
| Rust testi | 630 |
| Fonksiyonel senaryo | 546/546 |