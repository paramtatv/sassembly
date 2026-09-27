/* ── Language: Sanskrit by default, English and Hindi on request ──────────────
   SANSKRIT IS THE DEFAULT AND IT IS WHAT THE MARKUP CONTAINS. Every translatable
   node carries `data-i18n` (or `data-i18n-html` where the string holds markup),
   and the Sanskrit string is ALSO written inline in the HTML. So a visitor with
   no JavaScript, or a crawler, gets the Sanskrit site — not an empty shell of
   keys. The dictionary only ever REPLACES text that is already correct.

   Sassembly's own syntax is never translated. `मण्डलम्`, `वृत्तिः`, `अष्टकाः`,
   the type spellings and the register names are the language, not prose about
   it, so they read identically under all three settings — the way a C tutorial
   in Hindi still writes `int`.

   Devanagari serves both Sanskrit and Hindi, but not identically: the two have
   different conventions for conjuncts and for the shape of `ल`/`क्ष`. Tiro
   Devanagari Sanskrit and Tiro Devanagari Hindi are separate cuts for exactly
   that reason, and `site.css` selects between them on `html[lang]`.

   A NOTE THE AUTHOR OWES THE READER: the Sanskrit here is careful but it is not
   a native speaker's. Technical vocabulary is taken from the compiler's own
   corpus where it exists; the connective prose is composed. It wants review by
   someone who reads Sanskrit properly, and the keys make that a copy-edit of one
   file rather than a rewrite of five pages.
   ──────────────────────────────────────────────────────────────────────────── */
(function () {

  var DICT = {

    /* ── संस्कृतम् ──────────────────────────────────────────────────────── */
    sa: {
      'foot.discord': 'अध्ययनसङ्घः',
      'comm.h': 'सङ्कलकं सह पठामः',
      'comm.b': 'संस्कृतयन्त्रस्य विकासकानाम् अध्ययनसङ्घः Discord इत्यत्र — सङ्कलकः सह पठ्यते, प्रश्नाः पृच्छ्यन्ते, कार्यं च विभज्यते। आगन्तुं सर्वे स्वागताः।',
      'comm.cta': 'सङ्घे सम्मिल',
      'comm.note': 'यः सङ्कलकः स्वयं पठनीयः, सः सह पठनीयः अपि।',
      'lang.label': 'भाषा',
      'nav.learn': 'अध्ययनम्',
      'nav.reference': 'सन्दर्भः',
      'nav.playground': 'प्रयोगक्षेत्रम्',
      'nav.downloads': 'अवतरणम्',
      'nav.cta': 'साधनशृङ्खलां प्राप्नुहि',
      'nav.verify': 'रचनां परीक्षस्व',

      'home.title': 'संस्कृतयन्त्रम् — सव्याकरणो निर्देशसमुच्चयः',
      'home.eyebrow': 'शुल्कमुक्तं निर्देशसमुच्चयशिल्पम्',
      'home.h1': 'सव्याकरणो निर्देशसमुच्चयः।',
      'home.lede': 'संस्कृतयन्त्रं तन्त्रभाषा यस्याः रचना संस्कृतम्, अर्थश्च निर्देशसमुच्चयः। प्रत्येकस्य शिल्पनिर्देशस्य नाम अस्ति — अनामसङ्केतनाय मार्गो नास्ति। सङ्कलकः स्वयं संस्कृतयन्त्रेण लिखितः, स्वमेव सङ्कलयति, स्वकीयं बिम्बं च यथावत् पुनः रचयति।',
      'home.cta1': 'पठनम् आरभस्व',
      'home.cta2': 'प्रयोगक्षेत्रम् उद्घाटय',
      'home.fact1': 'RV64 लक्ष्यम्',
      'home.fact2': '१२ MiB साधनम्',
      'home.fact3': 'LLVM रहितम्',
      'home.pane.src': 'मूलम्',
      'home.pane.emit': 'निर्गतम्',

      'home.p1.h': 'शुल्कमुक्तं, रचनयैव',
      'home.p1.b': 'शिल्पं प्रकाशे प्रकटितम्, तस्य उपयोगे शुल्कं नास्ति। प्रत्येको निर्देशः शब्दकोशे नामितः; अनामितं सङ्केतनं लेखितुं व्याकरणे मार्गो नास्ति।',
      'home.p2.h': 'स्वाश्रयं, मूलपर्यन्तम्',
      'home.p2.b': 'एकविंशतिः मण्डलानि, ३८,३३५ पङ्क्तयश्च तान्येव एकविंशतिः मण्डलानि सङ्कलयन्ति। स्वरचनार्थं Rust, LLVM, बाह्यसंयोजकः वा न अपेक्ष्यते।',
      'home.p3.h': 'अष्टकपर्यन्तं परीक्ष्यम्',
      'home.p3.b': 'सङ्कलकेन सङ्कलितः सङ्कलकः तेनैव सङ्कलकेन सह अष्टकशः समानः। न केवलं समाना संस्करणसंज्ञा — तान्येव १३,९०,१७३ अष्टकानि, स्वयन्त्रे परीक्ष्याणि।',

      'colophon.eyebrow': 'पुष्पिका',
      'colophon.h': 'आरम्भस्य लेखः',
      'colophon.b': 'हस्तलिखितग्रन्थः अन्ते वदति केन कुतश्च लिखितः। अयमपि तथैव।',
      'colophon.f1': 'स्वसङ्कलिते बिम्बे अष्टकानि — द्वितीया अवस्था प्रथमया सह समाना',
      'colophon.f2': 'मण्डलानि सङ्कलितानि, एकस्मिन् बिम्बे संयोजितानि, यत् चलति शुद्धं च विरमति',
      'colophon.f3': 'सम्पूर्णं साधनम्, प्रचलितसाधनस्य प्रायः २०० MB इत्यस्य तुलनायाम्',
      'colophon.f4': 'सङ्कलके, संयोजके, आरोपके वा बाह्याश्रयाः',

      'aud.h': 'सर्वप्रकारकाय तन्त्रांशाय लिखितम्',
      'aud.note': 'प्रथमनिर्देशात् प्रयोगस्तरं यावत् एका भाषा',
      'aud.c1.h': 'कर्णकाः, स्थिरांशाश्च',
      'aud.c1.b': 'प्रथमनिर्देशादेव नग्नयन्त्रम्। न धावनकालः, न त्वया अलिखितः आवण्टकः, न गुप्तः पूर्वलेखः।',
      'aud.c2.h': 'सङ्कलकाः, धावनकालाश्च',
      'aud.c2.b': 'सन्दर्भसङ्कलकः एव उदाहरणम् — पदविभागः, व्याकरः, निर्णायकः, परीक्षकः, उत्सर्जकश्च, सर्वे पठनीयाः।',
      'aud.c3.h': 'अन्तर्निहितं, कालबद्धं च',
      'aud.c3.b': 'प्रत्येकम् आवण्टनं मूले लिखितम्। त्वया लिखितस्य अष्टकानां च मध्ये किमपि न योज्यते।',
      'aud.c4.h': 'प्रयोगतन्त्रांशः',
      'aud.c4.b': 'मण्डलानि, अभिलेखाः, अङ्कमालाः, प्रकारपरीक्षकश्च यः अनुमानं न करोति, प्रत्याख्यानं करोति।',
      'aud.c5.h': 'पुनरुत्पाद्या रचना',
      'aud.c5.b': 'स्थिरबिन्दुः भूमिः, न लक्ष्यम् — तान्येव मूलानि सर्वत्र तदेव बिम्बं जनयन्ति, परीक्षा च एकः आदेशः।',
      'aud.c6.h': 'अध्यापनं, अनुसन्धानं च',
      'aud.c6.b': 'सम्पूर्णः सङ्कलकः, आदितः अन्तं यावत् पठनीयः, सह औपचारिकव्याकरणेन यन्त्रप्रतिमानेन च।',

      'why.h': 'यन्त्राय स्वाभाविकं व्याकरणं कुतः?',
      'why.b1': 'पाणिनिना संस्कृतं प्रायः चतुःसहस्रसूत्रैः वर्णितम्, यानि अव्याकुलं संयुज्यन्ते — औपचारिकं व्याकरणं द्विसहस्रवर्षपूर्वम्। संस्कृतयन्त्रं ततः तदेव गृह्णाति यत् सङ्कलकाय उपयुक्तम्: विभक्तिः शब्दस्य कारकं वहति, अतः निर्देशस्य पदानि स्वस्वरूपं स्वयमेव कथयन्ति, न स्थानमात्रेण ज्ञायन्ते।',
      'why.b2': 'फलम् — संस्कृतयन्त्रस्य वाक्यं मनुष्याय व्याकराय च समानम् एव पठ्यते, दुष्टं वाक्यं च उभयोः समानकारणेन दुष्टम्।',
      'why.link': 'कारकचिह्नानि विस्तरेण →',
      'why.pane': 'कारकम्',

      'learn.title': 'कारकचिह्नानि — संस्कृतयन्त्रस्य लेखाः',
      'learn.eyebrow': 'भाषा',
      'learn.h1': 'कारकचिह्नानि',
      'learn.lede': 'कारकं नाम क्रियायां शब्दस्य भूमिका। संस्कृतयन्त्रं संस्कृतस्य विभक्तीः तां भूमिकां वोढुं प्रयुङ्क्ते, अतः पदं स्वस्वरूपं कथयति, न स्थानेन ज्ञायते।',
      'learn.h2a': 'स्थानं संकेतः; विभक्तिस्तु न',
      'learn.b2a': 'प्रायः संयोजकभाषासु प्रथमं पदं लक्ष्यम्, यतः केनचित् तथा निर्णीतम्, न तु पाठे किमपि तत् वदति। पदद्वयं विपरीतं कुरु — अन्यत्, वैधम्, अशुद्धं च कार्यक्रमं प्राप्नोषि। संस्कृतयन्त्रं भूमिकां स्थानात् अपनीय शब्दे स्थापयति।',
      'learn.tbl.cap': 'षट् कारकाणि, संस्कृतयन्त्रवाक्ये च प्रत्येकं किं सूचयति',
      'learn.th1': 'चिह्नम्', 'learn.th2': 'कारकम्', 'learn.th3': 'भूमिका', 'learn.th4': 'सूचयति',
      'learn.k1': 'कर्तृ', 'learn.k1d': 'क्रियां कुर्वती वृत्तिः निर्देशो वा',
      'learn.k2': 'कर्म', 'learn.k2d': 'यस्मिन् क्रिया — मूलपदम्',
      'learn.k3': 'करणम्', 'learn.k3d': 'येन साधनेन कोष्ठेन वा',
      'learn.k4': 'सम्प्रदानम्', 'learn.k4d': 'यस्मै फलं दीयते — लक्ष्यम्',
      'learn.k5': 'अपादानम्', 'learn.k5d': 'यस्मात् मूल्यं गृह्यते',
      'learn.k6': 'अधिकरणम्', 'learn.k6d': 'यस्मिन् स्थाने विस्तारे वा क्रिया',
      'learn.co1': 'कोष्ठः लिख्यते, न चिह्न्यते। प्रथमकोष्ठार्थं क१ इति लिख्य। #१ इति संस्कृतयन्त्रे कदापि वैधं न आसीत् — वर्णसमुच्चये तत्र चिह्नवर्णाय स्थानं नास्ति, अतः दोषः वर्णदोषः, न तु सम्भाव्यं किमपि सङ्कलयन् दुर्व्याकरः।',
      'learn.h2b': 'प्रकाराः चिह्निताः, न अनुमानेन व्याकुलाः',
      'learn.b2b': 'ॱॱ इति द्विचिह्नं प्रकारम् आरभते। अङ्कमाला — सन्ततः अवयवविस्तारः, यम् अन्यभाषाः slice इति वदन्ति — अङ्कः अन्तः इत्यनन्तरम् अवयवप्रकारेण लिख्यते।',
      'learn.b2c': 'अचिह्नितः प्रकारः सर्वत्र अचिह्नितः एव। इदं स्पष्टं भाति, अन्यभाषासु च दोषवर्गस्य मूलम्: दैर्घ्यं चिह्नितराशिना तुल्यते, २⁶³ अधिकपदे शेषः गृह्यते, स्थानान्तरणस्य विस्तारे च यन्त्रलक्ष्ययोः विवादः।',
      'learn.h2c': 'मण्डलानि, दृश्यता च',
      'learn.b2d': 'एका सञ्चिका मण्डलम् इत्यनेन एकं मण्डलं घोषयति। वृत्तिः गुप्ता यावत् सार्वजनिक इति न चिह्निता। मण्डलान्तरसन्दर्भः ॱ इति विभाजकेन, अतः पदविभागॱचिह्नक इति पदविभागस्य चिह्नकप्रकारं नामयति।',
      'learn.h2d': 'सङ्कलनकाले समावेशः',
      'learn.b2e': 'समावेशः सञ्चिकायाः अष्टकानि सङ्कलनकाले बिम्बे आनयति, मूल्यरूपेण। यत् नाम गृह्णाति तत् साधारणं नाम, प्रकाशितसारण्यां निर्णीतम् — न मार्गः। भाषायां कुत्रापि मार्गलेखनाय व्याकरणं नास्ति, अतः .. इति न प्रत्याख्यायते; तत् अलेख्यम् एव।',
      'learn.h2e': 'प्रत्याख्यानम्, न अनुमानम्',
      'learn.b2f': 'यत्र सङ्कलकः आकारं न अवतारयितुं शक्नोति, तत्र स कथयति स्थानं च नामयति। स किमपि सदृशं न उत्सृजति आशां च न करोति। अतः साधनं प्रत्याख्यातानाम् आकाराणां सङ्ख्यां वदति, असिद्धां पूर्णतां न दावयति।',
      'learn.li1': 'प्रत्याख्यानं वृत्तिं, रचनां, कारणं च नामयति।',
      'learn.li2': 'यस्य आकारस्य शाखा नास्ति तत् प्रत्याख्यानम्, न कदापि पूर्वनिर्धारितम्।',
      'learn.li3': 'नामितस्थाने पूर्वनिर्धारितं सङ्कलके दोषः, न गुणः।',
      'learn.next': 'निर्देशसन्दर्भं प्रति →',
      'learn.sn1': 'आरम्भः', 'learn.sn1a': 'साधनस्थापनम्', 'learn.sn1b': 'प्रथमं मण्डलम्', 'learn.sn1c': 'प्रयोगक्षेत्रम्',
      'learn.sn2': 'भाषा', 'learn.sn2b': 'प्रकाराः, अङ्कमालाश्च', 'learn.sn2c': 'मण्डलानि, दृश्यता', 'learn.sn2d': 'समावेशः', 'learn.sn2e': 'प्रत्याख्यानानि',
      'learn.sn3': 'यन्त्रम्', 'learn.sn3a': 'निर्देशसन्दर्भः', 'learn.sn3b': 'आदेशाः', 'learn.sn3c': 'औपचारिकव्याकरणम्',

      'ref.title': 'नियमाः — संस्कृतयन्त्रम्',
      'ref.eyebrow': 'शिल्पसन्दर्भः',
      'ref.h1': 'नियमाः',
      'ref.lede': 'प्रत्येकस्य शिल्पनिर्देशस्य संस्कृतयन्त्रनाम अस्ति। .insn 0x… इति निर्गममार्गो नास्ति, शब्दकोशेन अनामितं सङ्केतनं लेखितुं व्याकरणं न ददाति। अधः सम्पूर्णं तलम्।',
      'ref.d.h': 'आदेशाः',
      'ref.d.note': '॥ … ॥ इत्यनेन वेष्टिताः; एतदेव आदेशम् अभिव्यञ्जकात् पृथक् करोति',
      'ref.d.cap': 'उत्सर्जनस्य विन्यासस्य च आदेशाः, सह अपेक्षितपदसङ्ख्यया',
      'ref.d.th1': 'आदेशः', 'ref.d.th2': 'तुल्यम्', 'ref.d.th3': 'पदानि', 'ref.d.th4': 'उत्सृजति',
      'ref.d.r1': '८-बिट् मूल्यानि, प्रतिपदम् एकम्',
      'ref.d.r2': '६४-बिट् मूल्यानि, प्रतिपदम् एकम्',
      'ref.d.r3': 'नामितसारण्याः अष्टकानि, सङ्कलनकाले निर्णीतानि',
      'ref.d.r4': 'नामितं खण्डम् आरभते',
      'ref.d.r5': 'उत्सर्जनबिन्दुं संरेखयति',
      'ref.d.r6': 'सञ्चिकाष्टकरहितं स्थानम्',
      'ref.d.r7': 'संज्ञायाः विस्तारं लिखति',
      'ref.d.co': 'पदसूची मूल्यानि, न सङ्ख्या। ॥ अष्टकाः ३२ ॥ एकम् अष्टकं यस्य मूल्यं ३२ इति उत्सृजति — न द्वात्रिंशत् अष्टकानि। अङ्कमालायाः दैर्घ्यं समीपस्थेन अष्टाष्टकाः-पदेन वह्यते। हस्तेन निर्गतं पठतां सर्वाधिकः सामान्यदोषः एषः।',
      'ref.t.h': 'प्रकारलेखनम्',
      'ref.t.note': 'ॱॱ इत्यनेन आरब्धम्',
      'ref.t.cap1': 'एकांशाः', 'ref.t.cap2': 'संयुक्ताः',
      'ref.t.th1': 'लेखनम्', 'ref.t.th2': 'विस्तारः', 'ref.t.th3': 'चिह्नता', 'ref.t.th4': 'अर्थः',
      'ref.t.u': 'अचिह्नितम्', 'ref.t.s': 'चिह्नितम्', 'ref.t.ua': 'अचिह्नितम्, स्थानवत्',
      'ref.t.c1': 'T इत्यस्य अङ्कमाला — सन्तता, सदैर्घ्या',
      'ref.t.c2': 'सम्भाव्यः T',
      'ref.t.c3': 'रिक्ता अङ्कमाला',
      'ref.t.c4': 'शून्यमूल्यम्',
      'ref.g.h': 'मण्डलव्याकरणम्',
      'ref.g.note': 'प्रामाणिकं EBNF साधनेन सह आयाति',
      'ref.g.h3': 'व्याकरणेन निश्चितानि त्रीणि गुणानि',
      'ref.g.li1': 'खण्डाः सीमिताः, न अवकाशेन। आदि आरभते, इति समापयति। अवकाशस्य अर्थो नास्ति, अतः पुनर्विन्यासकः कार्यक्रमं परिवर्तयितुं न शक्नोति।',
      'ref.g.li2': 'वाक्यं । इत्यनेन समाप्यते। दण्डः समापकः; तस्य अभावे तत्रैव व्याकरदोषः, न त्रिपङ्क्तिपश्चात्।',
      'ref.g.li3': 'मार्गः लेखितुं न शक्यते। वर्णसमुच्चयः तं लेखितुं मार्गं न ददाति, अतः सञ्चिकाप्रवेशः निर्णीतनाम्ना एव।',
      'ref.g.co': 'टिप्पणी एकवर्णा। ॰ इति पङ्क्त्यन्तं यावत् टिप्पणीम् आरभते। तत् क्षेत्रविभाजकात् ॱ भिन्नः वर्णः, उभौ च एकसङ्केतान्तरे — साधने तस्यैव भ्रमस्य निवारकः अस्ति।',
      'ref.r.h': 'कोष्ठाः',
      'ref.r.note': 'क० — क३१ इति लिख्यन्ते; कदापि न चिह्न्यन्ते',
      'ref.r.cap': 'सामान्यकोष्ठसमूहः, सह प्रत्येकस्य RV64 नाम्ना',
      'ref.r.th1': 'संस्कृतयन्त्रम्', 'ref.r.th3': 'संकेतः',
      'ref.r.r1': 'दृढबद्धं शून्यम्', 'ref.r.r2': 'प्रतिगमनस्थानम्', 'ref.r.r3': 'स्तूपसूचकः',
      'ref.r.r4': 'रक्षितः, चौकटसूचकः', 'ref.r.r5': 'पदम्, प्रतिफलम्', 'ref.r.r6': 'पदम्, परिवेशाह्वानसङ्ख्या',

      'pg.title': 'प्रयोगक्षेत्रम् — संस्कृतयन्त्रम्',
      'pg.eyebrow': 'प्रयोगक्षेत्रम्',
      'pg.h1': 'संयोजय, चालय, अत्रैव',
      'pg.note': 'साधनं यन्त्रं च विचारके चलतः — तव लिखितं सङ्केत्य चाल्यते',
      'pg.loading': 'यन्त्रं भ्रियते…',
      'pg.sample': 'उदाहरणम्',
      'pg.run': 'चालय ▸',
      'pg.editable': 'सम्पाद्यम्',
      'pg.tab1': 'विवरणम्', 'pg.tab2': 'अष्टकानि', 'pg.tab3': 'चालनम्',
      'pg.c1.h': 'किमपि न गुप्तम्',
      'pg.c1.b': 'अष्टकपटलं तदेव वस्तु यत् साधनेन उत्सृष्टम् — न सुन्दरीकृतम् अनुमानम्। निषेधे सङ्केतकस्य स्वकीयं वचनम् एव दृश्यते, पङ्क्तिम् अष्टकं च नामयत्।',
      'pg.c2.h': 'यन्त्रं लघु',
      'pg.c2.b': 'यन्त्रम् इति RV64 प्रतिमानं यत् साधनेन सह आयाति। तत् आवृत्तीः विराममूल्यं च वदति, अतः कार्यक्रमस्य उत्तरं परीक्ष्या सङ्ख्या।',
      'pg.c3.h': 'सैव शृङ्खला, तव यन्त्रे',
      'pg.c3.b': 'एते एव द्वे विभागे ये स्थापितं साधनं चालयति, wasm-रूपेण सङ्कलिते। अत्र किमपि केवलविचारकमार्गो नास्ति।',
      'pg.foot': 'उदाहरणानि spec/*.sas इत्यस्मात् आगतानि; तव सम्पादितं तदेव सङ्केत्यते।',

      'dl.title': 'अवतरणम् — संस्कृतयन्त्रम्',
      'dl.eyebrow': 'अवतरणम्',
      'dl.h1': 'सम्पूर्णं साधनं १२ MiB',
      'dl.lede': 'सङ्कलकः, संयोजकः, आरोपकः, यन्त्रप्रतिमानं, प्रामाणिकव्याकरणं च। न सञ्चयप्रबन्धकः, न बाह्यसंयोजकः, तेन सह स्थापनीयं किमपि नास्ति।',
      'dl.src': 'मूलसङ्ग्रहः',
      'dl.srcmeta': '२१ मण्डलानि · ३८,३३५ पङ्क्तयः',
      'dl.browse': 'अवलोकय',
      'dl.srchash': 'सङ्कलकः, संस्कृतयन्त्रे, आदितः अन्तं यावत् पठनीयः',
      'dl.download': 'अवतारय',
      'dl.hash': 'sha256 [प्रकाशनेन सह प्रकाश्यम्]',
      'dl.v.eyebrow': 'परीक्षणम्',
      'dl.v.h': 'अस्मिन् पृष्ठे विश्वासं न कुरु। परीक्षस्व।',
      'dl.v.b': 'यत् प्रकाशनं पुनरुत्पादयितुं न शक्नोषि तत् श्रद्धया गृह्यते। त्रयः आदेशाः तां श्रद्धां स्थापयन्ति।',
      'dl.v.f1': 'अष्टकानि — यां सङ्ख्यां तव द्वितीयावस्था यथावत् मेलयेत्',
      'dl.v.f2': 'तृतीयावस्था द्वितीयया सह अष्टकशः समाना, न संस्करणसंज्ञामात्रम्',
      'dl.v.after': 'यदि तृतीयः पदः भेदं वदति, तर्हि तव यन्त्रे आरम्भः पुनरुत्पाद्यो नास्ति, फलं च दोषवृत्तं यत् वयम् इच्छामः। एकमपि भिन्नम् अष्टकं विफलता — सहनशीलता नास्ति, यतः यः सङ्कलकः स्वप्रवेशस्य प्रायः स्थिरं कार्यं, स न स्थिरं कार्यम्।',
      'dl.w.h': 'सञ्चिकायां किम् अस्ति',
      'dl.w.note': 'प्रत्येकम् अङ्गम्, अन्यत् किमपि न',
      'dl.w.cap': 'वितरणस्य अन्तर्गतम्, सह प्रत्येकस्य संस्कृतयन्त्रनाम्ना',
      'dl.w.th1': 'अङ्गम्', 'dl.w.th2': 'नाम', 'dl.w.th3': 'करोति',
      'dl.w.r1': 'चालकः: पदविभागः, व्याकरः, निर्णयः, प्रकारपरीक्षा, सङ्केतनम्, संयोजनम्',
      'dl.w.r2': 'RV64 यन्त्रप्रतिमानम् — बिम्बं चालयति विराममूल्यं च वदति',
      'dl.w.r3': 'सङ्कलकस्य स्वकीयानि २१ मण्डलानि, यानि उदाहरणमपि',
      'dl.w.r4': 'प्रामाणिकव्याकरणम्, शब्दकोशः, निर्देशसारण्यश्च',
      'dl.x1.h': 'मूलात् रचना',
      'dl.x1.b': 'सङ्ग्रहस्य रचनार्थं संस्कृतयन्त्रसङ्कलकः अपेक्ष्यते — एष एव आरम्भप्रश्नः। प्रकाशिता प्रथमावस्था वृत्तं भिनत्ति, उपरि तृतीयः पदः च तस्याः सत्यतां साधयति।',
      'dl.x2.h': 'अन्यलक्ष्यरचना',
      'dl.x2.b': 'यन्त्रं यत् किमपि भवतु, लक्ष्यं RV64 एव। macOS-रचना riscv64-रचना च समानमूलात् समानान्येव अष्टकानि उत्सृजतः — तदेव गुणं स्थिरबिन्दुः परीक्षते।',
      'dl.x3.h': 'दोषनिवेदनम्',
      'dl.x3.b': 'मूलम्, या अवस्था प्रत्याख्यातवती, यत् स्थानं च तया नामितम् — एतानि योजय। प्रत्याख्यानं सदैव स्थानं नामयति; यदि न अनामयत्, स एव दोषः।',

      'foot.home': 'गृहम्', 'foot.spec': 'नियमाः', 'foot.docs': 'लेखाः', 'foot.mirrors': 'प्रतिबिम्बानि',
      'foot.contrib': 'योगदानम्', 'foot.pg': 'प्रयोगक्षेत्रम्', 'foot.learnsyn': 'रचनाम् अधीयस्व', 'foot.ref': 'निर्देशसन्दर्भः',
      'foot.b1': 'शिल्पं, व्याकरणं, सन्दर्भसाधनं च प्रकाशे प्रकटितानि।',
      'foot.b2': 'सन्दर्भसाधनस्य लेखाः।',
      'foot.b3': 'प्रामाणिकव्याकरणं शब्दकोशश्च साधनेन सह आयातः, तेनैव सह संस्कृतौ।',
      'foot.b4': 'प्रत्येकेन प्रकाशनेन सह सङ्केताः प्रकाश्यन्ते, तैः सह हस्ताक्षराणि च।'
    },

    /* ── English ────────────────────────────────────────────────────────── */
    en: {
      'foot.discord': 'Discord',
      'comm.h': 'Read the compiler together',
      'comm.b': 'A study group for Sassembly developers on Discord — reading the compiler, asking questions, dividing the work. Everyone is welcome.',
      'comm.cta': 'Join the study group',
      'comm.note': 'a compiler meant to be read is a compiler meant to be read together',
      'lang.label': 'Language',
      'nav.learn': 'Learn', 'nav.reference': 'Reference', 'nav.playground': 'Playground',
      'nav.downloads': 'Downloads', 'nav.cta': 'Get the toolchain', 'nav.verify': 'Verify a build',

      'home.title': 'Sassembly — an instruction set with a grammar',
      'home.eyebrow': 'Royalty-free instruction set architecture',
      'home.h1': 'An instruction set with a grammar.',
      'home.lede': 'Sassembly is a systems language whose syntax is Sanskrit and whose semantics are an ISA. Every architectural instruction has a name — there is no escape hatch to a raw encoding. The compiler is written in Sassembly, compiles itself, and reproduces its own binary exactly.',
      'home.cta1': 'Start reading', 'home.cta2': 'Open the playground',
      'home.fact1': 'RV64 target', 'home.fact2': '12 MiB toolchain', 'home.fact3': 'No LLVM',
      'home.pane.src': 'source', 'home.pane.emit': 'emitted',

      'home.p1.h': 'Royalty-free, by construction',
      'home.p1.b': 'The architecture is published in the open and carries no licence on its use. Every instruction is named in the lexicon, and a program cannot smuggle in an unnamed encoding because the grammar has no syntax for one.',
      'home.p2.h': 'Self-hosting, all the way down',
      'home.p2.b': 'Twenty-one modules and 38,335 lines of Sassembly compile the twenty-one modules and 38,335 lines of Sassembly. The toolchain needs no Rust, no LLVM and no system assembler to build itself.',
      'home.p3.h': 'Verifiable to the octet',
      'home.p3.b': 'The compiler compiled by the compiler is byte-identical to the compiler that compiled it. Not a matching version string — the same 1,390,173 octets, checkable on your own machine in one command.',

      'colophon.eyebrow': 'Colophon', 'colophon.h': 'The bootstrap, stated',
      'colophon.b': 'A manuscript ends by recording who copied it and from what. So does this one.',
      'colophon.f1': 'octets in the self-compiled image — stage 2 identical to stage 1',
      'colophon.f2': 'corpus modules compiled and linked into one image that runs and halts clean',
      'colophon.f3': 'the entire toolchain, against roughly 200 MB for a conventional stack',
      'colophon.f4': 'third-party dependencies in the compiler, the linker or the loader',

      'aud.h': 'Written for every kind of software',
      'aud.note': 'one language from the reset vector to the application layer',
      'aud.c1.h': 'Kernels and firmware',
      'aud.c1.b': 'Bare metal from the first instruction. No runtime, no allocator you did not write, no prologue inserted on your behalf.',
      'aud.c2.h': 'Compilers and runtimes',
      'aud.c2.b': 'The reference compiler is the worked example: a lexer, parser, resolver, checker and emitter you can read end to end.',
      'aud.c3.h': 'Embedded and real-time',
      'aud.c3.b': 'Every allocation is written down in the source. Nothing is added between what you wrote and the octets that ship.',
      'aud.c4.h': 'Application software',
      'aud.c4.b': 'Modules, records, runs and a type checker that refuses rather than guesses. Ordinary programs, with ordinary tools.',
      'aud.c5.h': 'Reproducible builds',
      'aud.c5.b': 'A fixpoint is the floor, not the ambition: the same sources give the same image on any machine, and checking it is one command.',
      'aud.c6.h': 'Teaching and research',
      'aud.c6.b': 'A whole compiler, readable start to finish, with a formal grammar and a machine model small enough to hold in your head.',

      'why.h': 'Why a natural grammar for a machine?',
      'why.b1': 'Sanskrit was described by Pāṇini in about four thousand rules that compose without ambiguity — a formal grammar written down two millennia before the term existed. Sassembly borrows the part that matters to a compiler: case endings carry the role a word plays, so an instruction’s operands announce what they are rather than relying on position.',
      'why.b2': 'The practical consequence is that a Sassembly statement reads the same to a human and to the parser, and a malformed one is malformed for the same reason in both.',
      'why.link': 'The kāraka sigils, in full →', 'why.pane': 'operand roles',

      'learn.title': 'The kāraka sigils — Sassembly documentation',
      'learn.eyebrow': 'The language', 'learn.h1': 'Kāraka sigils',
      'learn.lede': 'A kāraka is the role a word plays in an action. Sassembly uses Sanskrit’s case endings to carry that role, so an operand states what it is rather than being identified by where it sits.',
      'learn.h2a': 'Position is a convention; a case ending is not',
      'learn.b2a': 'In most assembly languages the first operand is the destination because someone decided so, and nothing in the text says it. Reverse two operands and you get a different, valid, wrong program. Sassembly takes the role out of the position and puts it in the word.',
      'learn.tbl.cap': 'The six kārakas, and what each marks in a Sassembly statement',
      'learn.th1': 'Sigil', 'learn.th2': 'Kāraka', 'learn.th3': 'Role', 'learn.th4': 'Marks',
      'learn.k1': 'kartā', 'learn.k1d': 'the routine or instruction performing the action',
      'learn.k2': 'karma', 'learn.k2d': 'the value acted upon — a source operand',
      'learn.k3': 'karaṇa', 'learn.k3d': 'the register or means by which it is done',
      'learn.k4': 'sampradāna', 'learn.k4d': 'the destination the result is given to',
      'learn.k5': 'apādāna', 'learn.k5d': 'the place a value is taken from',
      'learn.k6': 'adhikaraṇa', 'learn.k6d': 'the address or span the action occurs within',
      'learn.co1': 'A register is spelled, never punctuated. Write क१ for register one. #१ is not valid Sassembly and never has been — the repertoire admits no sigil character there, so the mistake is a lexical error rather than a misparse that compiles into something plausible.',
      'learn.h2b': 'Types are marked, not inferred into ambiguity',
      'learn.b2b': 'The double marker ॱॱ introduces a type. A run — a contiguous span of elements, what other languages call a slice — is written अङ्कः अन्तः followed by its element type.',
      'learn.b2c': 'An unsigned type is unsigned everywhere it is used. This sounds obvious and is the source of a whole class of bug in languages where it is not: a length compared against a signed quantity, a remainder taken on a word above 2⁶³, a shift whose width the host and the target disagree about.',
      'learn.h2c': 'Modules and visibility',
      'learn.b2d': 'A file declares one module with मण्डलम्. A routine is private unless it is marked सार्वजनिक. Cross-module reference uses the separator ॱ, so पदविभागॱचिह्नक names the token type belonging to the lexer.',
      'learn.h2d': 'Compile-time embeds',
      'learn.b2e': 'समावेशः brings a file’s bytes into the image at compile time, as a value. The name it takes is an ordinary identifier resolved against a published table — not a path. There is deliberately no syntax for a path anywhere in the language, so a traversal like .. is not refused; it is unspellable.',
      'learn.h2e': 'Refusals, not guesses',
      'learn.b2f': 'Where the compiler cannot lower a shape, it says so and names the site. It does not emit something approximate and hope. This is why the toolchain reports a count of declined shapes rather than claiming completeness it cannot demonstrate.',
      'learn.li1': 'A refusal names the routine, the construct and the reason.',
      'learn.li2': 'A construct with no arm is a refusal, never a default.',
      'learn.li3': 'A default standing in for a named case is a defect in the compiler, not a feature.',
      'learn.next': 'Continue to the instruction reference →',
      'learn.sn1': 'Getting started', 'learn.sn1a': 'Install the toolchain', 'learn.sn1b': 'Your first module', 'learn.sn1c': 'The playground',
      'learn.sn2': 'The language', 'learn.sn2b': 'Types and runs', 'learn.sn2c': 'Modules and visibility', 'learn.sn2d': 'Compile-time embeds', 'learn.sn2e': 'Refusals, not guesses',
      'learn.sn3': 'The machine', 'learn.sn3a': 'Instruction reference', 'learn.sn3b': 'Directives', 'learn.sn3c': 'Formal grammar',

      'ref.title': 'Specification — Sassembly',
      'ref.eyebrow': 'Architecture reference', 'ref.h1': 'The specification',
      'ref.lede': 'Every architectural instruction has a Sassembly name. There is no .insn 0x… escape hatch, and the grammar provides no way to write an encoding the lexicon does not name. What follows is the whole of the surface.',
      'ref.d.h': 'Directives',
      'ref.d.note': 'wrapped in ॥ … ॥, which is what distinguishes a directive from an expression',
      'ref.d.cap': 'Emission and layout directives, with the operand count each requires',
      'ref.d.th1': 'Directive', 'ref.d.th2': 'Equivalent', 'ref.d.th3': 'Operands', 'ref.d.th4': 'Emits',
      'ref.d.r1': '8-bit values, one per operand', 'ref.d.r2': '64-bit values, one per operand',
      'ref.d.r3': 'a named table’s octets, resolved at compile time',
      'ref.d.r4': 'begins a named section', 'ref.d.r5': 'aligns the emission point',
      'ref.d.r6': 'address space with no file bytes', 'ref.d.r7': 'records a symbol’s extent',
      'ref.d.co': 'An operand list is values, not a count. ॥ अष्टकाः ३२ ॥ emits one octet whose value is 32 — it does not emit thirty-two octets. The length of a run is carried by the neighbouring अष्टाष्टकाः word. Misreading that is the single most common error when reading emitted output by hand.',
      'ref.t.h': 'Type spellings', 'ref.t.note': 'introduced by ॱॱ',
      'ref.t.cap1': 'Scalars', 'ref.t.cap2': 'Composites',
      'ref.t.th1': 'Spelling', 'ref.t.th2': 'Width', 'ref.t.th3': 'Signedness', 'ref.t.th4': 'Meaning',
      'ref.t.u': 'unsigned', 'ref.t.s': 'signed', 'ref.t.ua': 'unsigned, address-like',
      'ref.t.c1': 'a run of T — contiguous, with a length', 'ref.t.c2': 'an optional T',
      'ref.t.c3': 'the empty run', 'ref.t.c4': 'the nil value',
      'ref.g.h': 'Module grammar', 'ref.g.note': 'the normative EBNF ships with the toolchain',
      'ref.g.h3': 'Three properties the grammar guarantees',
      'ref.g.li1': 'Blocks are delimited, never indented. आदि opens and इति closes. Whitespace carries no meaning, so a reformatting tool cannot change a program.',
      'ref.g.li2': 'A statement ends in ।. The danda is the terminator; a missing one is a parse error at the point it is missing, not three lines later.',
      'ref.g.li3': 'A path cannot be written. The character repertoire admits no way to spell one, so file access is by resolved name only.',
      'ref.g.co': 'Comments are one glyph. ॰ begins a margin note that runs to the end of the line. It is a different character from the field separator ॱ, and they are one codepoint apart — the toolchain has a ratchet for exactly that confusion.',
      'ref.r.h': 'Registers', 'ref.r.note': 'spelled क० – क३१; never punctuated',
      'ref.r.cap': 'The general register file, with the RV64 name each maps to',
      'ref.r.th1': 'Sassembly', 'ref.r.th3': 'Convention',
      'ref.r.r1': 'hard-wired zero', 'ref.r.r2': 'return address', 'ref.r.r3': 'stack pointer',
      'ref.r.r4': 'saved, frame pointer', 'ref.r.r5': 'argument, return value', 'ref.r.r6': 'argument, environment call number',

      'pg.title': 'Playground — Sassembly',
      'pg.eyebrow': 'Playground', 'pg.h1': 'Assemble it, run it, right here',
      'pg.note': 'the assembler and the machine, compiled to wasm — it assembles what you typed',
      'pg.loading': 'loading the machine…', 'pg.sample': 'Sample',
      'pg.run': 'Run ▸', 'pg.editable': 'editable',
      'pg.tab1': 'details', 'pg.tab2': 'octets', 'pg.tab3': 'run',
      'pg.c1.h': 'Nothing is hidden',
      'pg.c1.b': 'The octets tab is the ELF the assembler emitted, not a pretty-printed guess. When it refuses you get its own diagnostics, naming a line and a byte.',
      'pg.c2.h': 'The machine is small',
      'pg.c2.b': 'यन्त्रम् is the RV64 model the toolchain ships. It reports cycles and a halt value, so a program’s answer is a number you can assert on.',
      'pg.c3.h': 'Same chain, your machine',
      'pg.c3.b': 'These are the same two crates the installed toolchain uses, compiled to wasm. Nothing here is a browser-only path.',
      'pg.foot': 'The samples come from spec/*.sas; what runs is what you edited.',

      'dl.title': 'Downloads — Sassembly',
      'dl.eyebrow': 'Downloads', 'dl.h1': 'The whole toolchain is 12 MiB',
      'dl.lede': 'Compiler, linker, loader, machine model and the normative grammar. No package manager, no system assembler, and nothing to install beside it.',
      'dl.src': 'Source corpus', 'dl.srcmeta': '21 modules · 38,335 lines', 'dl.browse': 'Browse',
      'dl.srchash': 'the compiler, in Sassembly, readable end to end',
      'dl.download': 'Download', 'dl.hash': 'sha256 [PUBLISHED WITH THE RELEASE]',
      'dl.v.eyebrow': 'Verification', 'dl.v.h': 'Do not trust this page. Check it.',
      'dl.v.b': 'A release you cannot reproduce is a release you are taking on faith. Three commands replace the faith.',
      'dl.v.f1': 'octets — the figure your stage 2 must match exactly',
      'dl.v.f2': 'stage 3 against stage 2, not a version string',
      'dl.v.after': 'If step 3 reports a difference, the bootstrap is not reproducible on your machine and the result is a bug report we want. A single differing octet is a failure — there is no tolerance, because a compiler that is nearly a stable function of its input is not one.',
      'dl.w.h': 'What is in the archive', 'dl.w.note': 'every component, and nothing else',
      'dl.w.cap': 'Contents of the distribution, with the Sassembly name of each component',
      'dl.w.th1': 'Component', 'dl.w.th2': 'Name', 'dl.w.th3': 'Does',
      'dl.w.r1': 'the driver: lex, parse, resolve, typecheck, encode, link',
      'dl.w.r2': 'the RV64 machine model — runs an image and reports its halt value',
      'dl.w.r3': 'the compiler’s own 21 modules, which are also the worked example',
      'dl.w.r4': 'the normative grammar, the lexicon and the instruction tables',
      'dl.x1.h': 'Building from source',
      'dl.x1.b': 'The corpus needs a Sassembly compiler to build, which is the usual bootstrap question. A published stage 1 image breaks the circle, and step 3 above proves it was honest.',
      'dl.x2.h': 'Cross-compiling',
      'dl.x2.b': 'The target is RV64 regardless of the host. A macOS build and a riscv64 build emit the same octets for the same source — that is the same property the fixpoint checks.',
      'dl.x3.h': 'Reporting a defect',
      'dl.x3.b': 'Include the source, the stage that refused, and the site it named. A refusal always names one; if it did not, that is the defect.',

      'foot.home': 'Home', 'foot.spec': 'Specification', 'foot.docs': 'Documentation', 'foot.mirrors': 'Mirrors',
      'foot.contrib': 'Contributing', 'foot.pg': 'Playground', 'foot.learnsyn': 'Learn the syntax', 'foot.ref': 'Instruction reference',
      'foot.b1': 'The architecture, the grammar and the reference toolchain are published in the open.',
      'foot.b2': 'Documentation for the reference toolchain.',
      'foot.b3': 'The normative grammar and lexicon ship with the toolchain and are versioned with it.',
      'foot.b4': 'Checksums are published with each release and signed alongside it.'
    },

    /* ── हिन्दी ─────────────────────────────────────────────────────────── */
    hi: {
      'foot.discord': 'डिस्कॉर्ड',
      'comm.h': 'कंपाइलर साथ मिलकर पढ़ें',
      'comm.b': 'Sassembly डेवलपर्स का अध्ययन समूह Discord पर — कंपाइलर पढ़ना, प्रश्न पूछना, काम बाँटना। सब सादर आमंत्रित हैं।',
      'comm.cta': 'समूह में शामिल हों',
      'comm.note': 'जो कंपाइलर पढ़े जाने के लिए बना है, वह साथ पढ़े जाने के लिए भी बना है',
      'lang.label': 'भाषा',
      'nav.learn': 'सीखें', 'nav.reference': 'संदर्भ', 'nav.playground': 'प्रयोगशाला',
      'nav.downloads': 'डाउनलोड', 'nav.cta': 'टूलचेन प्राप्त करें', 'nav.verify': 'बिल्ड जाँचें',

      'home.title': 'Sassembly — व्याकरण वाला निर्देश-समुच्चय',
      'home.eyebrow': 'रॉयल्टी-मुक्त निर्देश-समुच्चय आर्किटेक्चर',
      'home.h1': 'एक निर्देश-समुच्चय, अपने व्याकरण के साथ।',
      'home.lede': 'Sassembly एक सिस्टम भाषा है जिसका वाक्य-विन्यास संस्कृत है और जिसका अर्थ एक ISA है। हर आर्किटेक्चरल निर्देश का एक नाम है — किसी कच्चे एन्कोडिंग तक पहुँचने का कोई रास्ता नहीं। कंपाइलर स्वयं Sassembly में लिखा है, स्वयं को कंपाइल करता है, और अपनी ही बाइनरी को बिलकुल वैसा ही दोबारा बनाता है।',
      'home.cta1': 'पढ़ना शुरू करें', 'home.cta2': 'प्रयोगशाला खोलें',
      'home.fact1': 'RV64 लक्ष्य', 'home.fact2': '12 MiB टूलचेन', 'home.fact3': 'LLVM नहीं',
      'home.pane.src': 'स्रोत', 'home.pane.emit': 'निर्गत',

      'home.p1.h': 'रॉयल्टी-मुक्त, रचना से ही',
      'home.p1.b': 'आर्किटेक्चर खुले में प्रकाशित है और उसके उपयोग पर कोई लाइसेंस नहीं। हर निर्देश शब्दकोश में नामित है, और कोई प्रोग्राम बिना नाम वाली एन्कोडिंग नहीं घुसा सकता, क्योंकि व्याकरण में उसके लिए कोई वाक्य-रचना ही नहीं है।',
      'home.p2.h': 'स्व-आश्रित, जड़ तक',
      'home.p2.b': 'इक्कीस मॉड्यूल और 38,335 पंक्तियाँ उन्हीं इक्कीस मॉड्यूल और 38,335 पंक्तियों को कंपाइल करती हैं। स्वयं को बनाने के लिए टूलचेन को न Rust चाहिए, न LLVM, न कोई सिस्टम असेंबलर।',
      'home.p3.h': 'ऑक्टेट तक जाँच-योग्य',
      'home.p3.b': 'कंपाइलर द्वारा कंपाइल किया गया कंपाइलर, उसी कंपाइलर के बाइट-दर-बाइट समान है। कोई मिलता-जुलता संस्करण-नाम नहीं — वही 1,390,173 ऑक्टेट, आपकी ही मशीन पर एक आदेश से जाँचे जा सकते हैं।',

      'colophon.eyebrow': 'पुष्पिका', 'colophon.h': 'बूटस्ट्रैप, स्पष्ट शब्दों में',
      'colophon.b': 'हस्तलिखित ग्रंथ अंत में बताता है कि उसे किसने और किससे नकल किया। यह भी वही करता है।',
      'colophon.f1': 'स्व-कंपाइल की गई इमेज में ऑक्टेट — चरण 2, चरण 1 के समान',
      'colophon.f2': 'मॉड्यूल कंपाइल और लिंक होकर एक इमेज बने, जो चलती है और साफ़ रुकती है',
      'colophon.f3': 'पूरी टूलचेन, परंपरागत स्टैक के लगभग 200 MB के मुक़ाबले',
      'colophon.f4': 'कंपाइलर, लिंकर या लोडर में बाहरी निर्भरताएँ',

      'aud.h': 'हर तरह के सॉफ़्टवेयर के लिए',
      'aud.note': 'रीसेट वेक्टर से एप्लिकेशन स्तर तक, एक ही भाषा',
      'aud.c1.h': 'कर्नेल और फ़र्मवेयर',
      'aud.c1.b': 'पहले निर्देश से ही बेयर-मेटल। कोई रनटाइम नहीं, कोई ऐसा एलोकेटर नहीं जो आपने न लिखा हो, कोई छिपा प्रस्तावना-कोड नहीं।',
      'aud.c2.h': 'कंपाइलर और रनटाइम',
      'aud.c2.b': 'संदर्भ कंपाइलर स्वयं उदाहरण है: लेक्सर, पार्सर, रिज़ॉल्वर, चेकर और एमिटर — सब आदि से अंत तक पढ़े जा सकते हैं।',
      'aud.c3.h': 'एम्बेडेड और रियल-टाइम',
      'aud.c3.b': 'हर एलोकेशन स्रोत में लिखा है। आपने जो लिखा और जो ऑक्टेट भेजे जाते हैं, उनके बीच कुछ नहीं जोड़ा जाता।',
      'aud.c4.h': 'एप्लिकेशन सॉफ़्टवेयर',
      'aud.c4.b': 'मॉड्यूल, रिकॉर्ड, रन और एक टाइप-चेकर जो अनुमान नहीं लगाता, मना करता है। सामान्य प्रोग्राम, सामान्य औज़ार।',
      'aud.c5.h': 'पुनरुत्पाद्य बिल्ड',
      'aud.c5.b': 'फ़िक्सपॉइंट ज़मीन है, लक्ष्य नहीं: वही स्रोत किसी भी मशीन पर वही इमेज देते हैं, और जाँच एक आदेश है।',
      'aud.c6.h': 'शिक्षण और शोध',
      'aud.c6.b': 'एक पूरा कंपाइलर, आदि से अंत तक पढ़ने योग्य, औपचारिक व्याकरण और इतने छोटे मशीन-मॉडल के साथ कि वह दिमाग़ में समा जाए।',

      'why.h': 'मशीन के लिए प्राकृतिक व्याकरण क्यों?',
      'why.b1': 'पाणिनि ने संस्कृत का वर्णन लगभग चार हज़ार सूत्रों में किया, जो बिना अस्पष्टता जुड़ते हैं — एक औपचारिक व्याकरण, उस शब्द के बनने से दो सहस्राब्दी पहले। Sassembly उसमें से वही लेता है जो कंपाइलर के काम का है: विभक्ति शब्द की भूमिका ढोती है, इसलिए निर्देश के ऑपरैंड स्वयं बताते हैं कि वे क्या हैं, स्थान पर निर्भर नहीं रहते।',
      'why.b2': 'व्यावहारिक परिणाम यह है कि Sassembly का एक वाक्य मनुष्य और पार्सर को एक-सा पढ़ता है, और ग़लत वाक्य दोनों के लिए एक ही कारण से ग़लत है।',
      'why.link': 'कारक चिह्न, विस्तार से →', 'why.pane': 'ऑपरैंड भूमिकाएँ',

      'learn.title': 'कारक चिह्न — Sassembly दस्तावेज़',
      'learn.eyebrow': 'भाषा', 'learn.h1': 'कारक चिह्न',
      'learn.lede': 'कारक का अर्थ है क्रिया में शब्द की भूमिका। Sassembly संस्कृत की विभक्तियों से वह भूमिका ढोता है, इसलिए ऑपरैंड बताता है कि वह क्या है, यह नहीं कि वह कहाँ बैठा है।',
      'learn.h2a': 'स्थान एक परंपरा है; विभक्ति नहीं',
      'learn.b2a': 'अधिकतर असेंबली भाषाओं में पहला ऑपरैंड गंतव्य होता है क्योंकि किसी ने तय कर दिया, और पाठ में कुछ भी यह नहीं कहता। दो ऑपरैंड उलट दें और आपको एक भिन्न, वैध, ग़लत प्रोग्राम मिलता है। Sassembly भूमिका को स्थान से हटाकर शब्द में रख देता है।',
      'learn.tbl.cap': 'छह कारक, और Sassembly वाक्य में हर एक क्या चिह्नित करता है',
      'learn.th1': 'चिह्न', 'learn.th2': 'कारक', 'learn.th3': 'भूमिका', 'learn.th4': 'चिह्नित करता है',
      'learn.k1': 'कर्ता', 'learn.k1d': 'वह रूटीन या निर्देश जो क्रिया कर रहा है',
      'learn.k2': 'कर्म', 'learn.k2d': 'जिस पर क्रिया हो — स्रोत ऑपरैंड',
      'learn.k3': 'करण', 'learn.k3d': 'वह रजिस्टर या साधन जिससे किया जाए',
      'learn.k4': 'सम्प्रदान', 'learn.k4d': 'वह गंतव्य जिसे परिणाम दिया जाए',
      'learn.k5': 'अपादान', 'learn.k5d': 'वह जगह जहाँ से मान लिया जाए',
      'learn.k6': 'अधिकरण', 'learn.k6d': 'वह पता या विस्तार जिसके भीतर क्रिया हो',
      'learn.co1': 'रजिस्टर लिखा जाता है, विरामचिह्न से नहीं। रजिस्टर एक के लिए क१ लिखें। #१ Sassembly में वैध नहीं है और कभी नहीं था — वर्ण-समुच्चय वहाँ किसी चिह्न-वर्ण को स्वीकार नहीं करता, इसलिए यह एक लेक्सिकल त्रुटि है, न कि ऐसा ग़लत पार्स जो कुछ संभव-सा कंपाइल कर दे।',
      'learn.h2b': 'टाइप चिह्नित होते हैं, अनुमान से अस्पष्ट नहीं',
      'learn.b2b': 'दोहरा चिह्न ॱॱ एक टाइप शुरू करता है। एक रन — तत्वों का सतत विस्तार, जिसे अन्य भाषाएँ slice कहती हैं — अङ्कः अन्तः के बाद उसके तत्व-टाइप के साथ लिखा जाता है।',
      'learn.b2c': 'अचिह्नित टाइप हर जगह अचिह्नित ही है। यह स्पष्ट लगता है और जिन भाषाओं में ऐसा नहीं, वहाँ बग़ों के एक पूरे वर्ग का स्रोत है: लंबाई की तुलना चिह्नित राशि से, 2⁶³ से बड़े शब्द पर शेषफल, और ऐसी शिफ़्ट जिसकी चौड़ाई पर होस्ट और लक्ष्य असहमत हों।',
      'learn.h2c': 'मॉड्यूल और दृश्यता',
      'learn.b2d': 'एक फ़ाइल मण्डलम् से एक मॉड्यूल घोषित करती है। रूटीन निजी है जब तक सार्वजनिक चिह्नित न हो। मॉड्यूल-पार संदर्भ विभाजक ॱ से होता है, इसलिए पदविभागॱचिह्नक लेक्सर के टोकन-टाइप को नामित करता है।',
      'learn.h2d': 'कंपाइल-समय एम्बेड',
      'learn.b2e': 'समावेशः किसी फ़ाइल के बाइट कंपाइल-समय पर इमेज में, एक मान के रूप में ले आता है। जो नाम वह लेता है वह एक सामान्य पहचानकर्ता है, प्रकाशित तालिका के विरुद्ध हल किया गया — कोई पथ नहीं। भाषा में कहीं भी पथ लिखने की वाक्य-रचना जान-बूझकर नहीं है, इसलिए .. को मना नहीं किया जाता; वह लिखा ही नहीं जा सकता।',
      'learn.h2e': 'मनाही, अनुमान नहीं',
      'learn.b2f': 'जहाँ कंपाइलर किसी आकृति को नीचे नहीं उतार सकता, वह कहता है और स्थान नामित करता है। वह कुछ लगभग-सा नहीं भेजता और आशा नहीं करता। इसीलिए टूलचेन मना की गई आकृतियों की गिनती बताता है, ऐसी पूर्णता का दावा नहीं करता जो वह दिखा न सके।',
      'learn.li1': 'मनाही रूटीन, रचना और कारण को नामित करती है।',
      'learn.li2': 'जिस रचना के लिए कोई शाखा नहीं, वह मनाही है — कभी डिफ़ॉल्ट नहीं।',
      'learn.li3': 'किसी नामित स्थिति की जगह खड़ा डिफ़ॉल्ट कंपाइलर में दोष है, विशेषता नहीं।',
      'learn.next': 'निर्देश संदर्भ की ओर →',
      'learn.sn1': 'शुरुआत', 'learn.sn1a': 'टूलचेन स्थापित करें', 'learn.sn1b': 'आपका पहला मॉड्यूल', 'learn.sn1c': 'प्रयोगशाला',
      'learn.sn2': 'भाषा', 'learn.sn2b': 'टाइप और रन', 'learn.sn2c': 'मॉड्यूल और दृश्यता', 'learn.sn2d': 'कंपाइल-समय एम्बेड', 'learn.sn2e': 'मनाही, अनुमान नहीं',
      'learn.sn3': 'मशीन', 'learn.sn3a': 'निर्देश संदर्भ', 'learn.sn3b': 'निर्देशक', 'learn.sn3c': 'औपचारिक व्याकरण',

      'ref.title': 'विशिष्टि — Sassembly',
      'ref.eyebrow': 'आर्किटेक्चर संदर्भ', 'ref.h1': 'विशिष्टि',
      'ref.lede': 'हर आर्किटेक्चरल निर्देश का एक Sassembly नाम है। कोई .insn 0x… बचाव-मार्ग नहीं, और व्याकरण ऐसी एन्कोडिंग लिखने का कोई रास्ता नहीं देता जिसे शब्दकोश नामित न करे। आगे जो है, वही पूरी सतह है।',
      'ref.d.h': 'निर्देशक',
      'ref.d.note': '॥ … ॥ में लपेटे हुए; यही निर्देशक को अभिव्यक्ति से अलग करता है',
      'ref.d.cap': 'निर्गम और विन्यास निर्देशक, हर एक की अपेक्षित ऑपरैंड-संख्या के साथ',
      'ref.d.th1': 'निर्देशक', 'ref.d.th2': 'समतुल्य', 'ref.d.th3': 'ऑपरैंड', 'ref.d.th4': 'भेजता है',
      'ref.d.r1': '8-बिट मान, प्रति ऑपरैंड एक', 'ref.d.r2': '64-बिट मान, प्रति ऑपरैंड एक',
      'ref.d.r3': 'नामित तालिका के ऑक्टेट, कंपाइल-समय पर हल',
      'ref.d.r4': 'नामित खंड शुरू करता है', 'ref.d.r5': 'निर्गम-बिंदु संरेखित करता है',
      'ref.d.r6': 'फ़ाइल-बाइट रहित पता-स्थान', 'ref.d.r7': 'किसी प्रतीक का विस्तार दर्ज करता है',
      'ref.d.co': 'ऑपरैंड-सूची मान हैं, गिनती नहीं। ॥ अष्टकाः ३२ ॥ एक ऑक्टेट भेजता है जिसका मान 32 है — बत्तीस ऑक्टेट नहीं। रन की लंबाई पड़ोसी अष्टाष्टकाः शब्द ढोता है। हाथ से निर्गत पढ़ते समय यही सबसे आम भूल है।',
      'ref.t.h': 'टाइप लेखन', 'ref.t.note': 'ॱॱ से आरंभ',
      'ref.t.cap1': 'अदिश', 'ref.t.cap2': 'संयुक्त',
      'ref.t.th1': 'लेखन', 'ref.t.th2': 'चौड़ाई', 'ref.t.th3': 'चिह्नता', 'ref.t.th4': 'अर्थ',
      'ref.t.u': 'अचिह्नित', 'ref.t.s': 'चिह्नित', 'ref.t.ua': 'अचिह्नित, पता-जैसा',
      'ref.t.c1': 'T का एक रन — सतत, लंबाई के साथ', 'ref.t.c2': 'वैकल्पिक T',
      'ref.t.c3': 'रिक्त रन', 'ref.t.c4': 'शून्य मान',
      'ref.g.h': 'मॉड्यूल व्याकरण', 'ref.g.note': 'प्रामाणिक EBNF टूलचेन के साथ आता है',
      'ref.g.h3': 'व्याकरण जिन तीन गुणों की गारंटी देता है',
      'ref.g.li1': 'खंड सीमांकित हैं, इंडेंट से नहीं। आदि खोलता है, इति बंद करता है। रिक्त-स्थान का कोई अर्थ नहीं, इसलिए कोई फ़ॉर्मैटर प्रोग्राम बदल नहीं सकता।',
      'ref.g.li2': 'वाक्य । पर समाप्त होता है। दंड समापक है; उसका अभाव वहीं पार्स-त्रुटि है जहाँ वह नहीं है, तीन पंक्ति बाद नहीं।',
      'ref.g.li3': 'पथ लिखा नहीं जा सकता। वर्ण-समुच्चय उसे लिखने का कोई रास्ता नहीं देता, इसलिए फ़ाइल-पहुँच केवल हल किए गए नाम से।',
      'ref.g.co': 'टिप्पणी एक ही चिह्न है। ॰ से पंक्ति के अंत तक चलने वाली हाशिया-टिप्पणी शुरू होती है। यह क्षेत्र-विभाजक ॱ से भिन्न वर्ण है, और दोनों एक कोडपॉइंट की दूरी पर हैं — टूलचेन में ठीक उसी भ्रम के लिए एक रैचेट है।',
      'ref.r.h': 'रजिस्टर', 'ref.r.note': 'क० – क३१ लिखे जाते हैं; कभी विरामचिह्न से नहीं',
      'ref.r.cap': 'सामान्य रजिस्टर-फ़ाइल, हर एक के RV64 नाम के साथ',
      'ref.r.th1': 'Sassembly', 'ref.r.th3': 'परंपरा',
      'ref.r.r1': 'स्थायी शून्य', 'ref.r.r2': 'वापसी पता', 'ref.r.r3': 'स्टैक पॉइंटर',
      'ref.r.r4': 'संरक्षित, फ़्रेम पॉइंटर', 'ref.r.r5': 'तर्क, वापसी मान', 'ref.r.r6': 'तर्क, पर्यावरण-कॉल संख्या',

      'pg.title': 'प्रयोगशाला — Sassembly',
      'pg.eyebrow': 'प्रयोगशाला', 'pg.h1': 'यहीं असेंबल करें, यहीं चलाएँ',
      'pg.note': 'असेंबलर और मशीन, wasm में संकलित — जो आपने लिखा वही असेंबल होता है',
      'pg.loading': 'मशीन लोड हो रही है…', 'pg.sample': 'नमूना',
      'pg.run': 'चलाएँ ▸', 'pg.editable': 'संपाद्य',
      'pg.tab1': 'विवरण', 'pg.tab2': 'ऑक्टेट', 'pg.tab3': 'चलाना',
      'pg.c1.h': 'कुछ छिपा नहीं है',
      'pg.c1.b': 'ऑक्टेट टैब वही ELF है जो असेंबलर ने बनाया, कोई सजाया हुआ अनुमान नहीं। मना करने पर उसका अपना संदेश मिलता है, पंक्ति और बाइट के साथ।',
      'pg.c2.h': 'मशीन छोटी है',
      'pg.c2.b': 'यन्त्रम् वह RV64 मॉडल है जो टूलचेन के साथ आता है। यह चक्र और एक विराम-मान बताता है, इसलिए प्रोग्राम का उत्तर एक संख्या है जिस पर आप दावा कर सकते हैं।',
      'pg.c3.h': 'वही चेन, आपकी मशीन',
      'pg.c3.b': 'ये वही दो क्रेट हैं जो स्थापित टूलचेन इस्तेमाल करता है, wasm में संकलित। यहाँ कुछ भी केवल-ब्राउज़र का रास्ता नहीं है।',
      'pg.foot': 'नमूने spec/*.sas से आते हैं; जो चलता है वही है जो आपने संपादित किया।',

      'dl.title': 'डाउनलोड — Sassembly',
      'dl.eyebrow': 'डाउनलोड', 'dl.h1': 'पूरी टूलचेन 12 MiB है',
      'dl.lede': 'कंपाइलर, लिंकर, लोडर, मशीन-मॉडल और प्रामाणिक व्याकरण। कोई पैकेज मैनेजर नहीं, कोई सिस्टम असेंबलर नहीं, और साथ में स्थापित करने को कुछ नहीं।',
      'dl.src': 'स्रोत संग्रह', 'dl.srcmeta': '21 मॉड्यूल · 38,335 पंक्तियाँ', 'dl.browse': 'देखें',
      'dl.srchash': 'कंपाइलर, Sassembly में, आदि से अंत तक पढ़ने योग्य',
      'dl.download': 'डाउनलोड', 'dl.hash': 'sha256 [रिलीज़ के साथ प्रकाशित]',
      'dl.v.eyebrow': 'सत्यापन', 'dl.v.h': 'इस पृष्ठ पर भरोसा न करें। जाँचें।',
      'dl.v.b': 'जिस रिलीज़ को आप दोबारा नहीं बना सकते, उसे आप श्रद्धा से ले रहे हैं। तीन आदेश उस श्रद्धा की जगह ले लेते हैं।',
      'dl.v.f1': 'ऑक्टेट — वह संख्या जिससे आपका चरण 2 बिलकुल मेल खाना चाहिए',
      'dl.v.f2': 'चरण 3, चरण 2 के मुक़ाबले — कोई संस्करण-नाम नहीं',
      'dl.v.after': 'यदि तीसरा चरण कोई अंतर बताता है, तो आपकी मशीन पर बूटस्ट्रैप पुनरुत्पाद्य नहीं है, और वह परिणाम एक बग-रिपोर्ट है जो हम चाहते हैं। एक भी भिन्न ऑक्टेट विफलता है — कोई छूट नहीं, क्योंकि जो कंपाइलर अपने इनपुट का लगभग स्थिर फलन है, वह स्थिर फलन नहीं है।',
      'dl.w.h': 'संग्रह में क्या है', 'dl.w.note': 'हर घटक, और कुछ नहीं',
      'dl.w.cap': 'वितरण की सामग्री, हर घटक के Sassembly नाम के साथ',
      'dl.w.th1': 'घटक', 'dl.w.th2': 'नाम', 'dl.w.th3': 'करता है',
      'dl.w.r1': 'ड्राइवर: लेक्स, पार्स, रिज़ॉल्व, टाइपचेक, एनकोड, लिंक',
      'dl.w.r2': 'RV64 मशीन-मॉडल — इमेज चलाता है और उसका विराम-मान बताता है',
      'dl.w.r3': 'कंपाइलर के अपने 21 मॉड्यूल, जो उदाहरण भी हैं',
      'dl.w.r4': 'प्रामाणिक व्याकरण, शब्दकोश और निर्देश-तालिकाएँ',
      'dl.x1.h': 'स्रोत से बनाना',
      'dl.x1.b': 'संग्रह को बनाने के लिए एक Sassembly कंपाइलर चाहिए — यही आम बूटस्ट्रैप प्रश्न है। एक प्रकाशित चरण-1 इमेज इस चक्र को तोड़ती है, और ऊपर का तीसरा चरण सिद्ध करता है कि वह ईमानदार थी।',
      'dl.x2.h': 'क्रॉस-कंपाइल',
      'dl.x2.b': 'होस्ट जो भी हो, लक्ष्य RV64 है। एक macOS बिल्ड और एक riscv64 बिल्ड एक ही स्रोत के लिए वही ऑक्टेट भेजते हैं — फ़िक्सपॉइंट भी वही गुण जाँचता है।',
      'dl.x3.h': 'दोष की सूचना',
      'dl.x3.b': 'स्रोत, वह चरण जिसने मना किया, और उसने जो स्थान नामित किया — ये शामिल करें। मनाही सदा एक स्थान नामित करती है; यदि न किया, तो वही दोष है।',

      'foot.home': 'मुख्य पृष्ठ', 'foot.spec': 'विशिष्टि', 'foot.docs': 'दस्तावेज़', 'foot.mirrors': 'मिरर',
      'foot.contrib': 'योगदान', 'foot.pg': 'प्रयोगशाला', 'foot.learnsyn': 'वाक्य-विन्यास सीखें', 'foot.ref': 'निर्देश संदर्भ',
      'foot.b1': 'आर्किटेक्चर, व्याकरण और संदर्भ टूलचेन खुले में प्रकाशित हैं।',
      'foot.b2': 'संदर्भ टूलचेन के दस्तावेज़।',
      'foot.b3': 'प्रामाणिक व्याकरण और शब्दकोश टूलचेन के साथ आते हैं और उसी के साथ संस्करणित हैं।',
      'foot.b4': 'हर रिलीज़ के साथ चेकसम प्रकाशित और हस्ताक्षरित होते हैं।'
    }
  };

  var KEY = 'sassembly-lang';
  var DEFAULT = 'sa';
  var NAMES = { sa: 'संस्कृतम्', en: 'English', hi: 'हिन्दी' };

  function stored() {
    try { return localStorage.getItem(KEY); } catch (e) { return null; }
  }
  function remember(v) {
    try { localStorage.setItem(KEY, v); } catch (e) { /* private window: this page only */ }
  }

  function apply(lang) {
    var d = DICT[lang] || DICT[DEFAULT];
    document.documentElement.setAttribute('lang', lang);

    var nodes = document.querySelectorAll('[data-i18n]');
    for (var i = 0; i < nodes.length; i++) {
      var k = nodes[i].getAttribute('data-i18n');
      if (d[k] != null) { nodes[i].textContent = d[k]; }
    }
    var html = document.querySelectorAll('[data-i18n-html]');
    for (var j = 0; j < html.length; j++) {
      var hk = html[j].getAttribute('data-i18n-html');
      if (d[hk] != null) { html[j].innerHTML = d[hk]; }
    }
    var t = document.querySelector('[data-i18n-title]');
    if (t) {
      var tk = t.getAttribute('data-i18n-title');
      if (d[tk] != null) { document.title = d[tk]; }
    }
    var sels = document.querySelectorAll('.langpick');
    for (var s = 0; s < sels.length; s++) { sels[s].value = lang; }
  }

  window.SassemblyLang = { apply: apply, names: NAMES, dict: DICT, fallback: DEFAULT };

  document.addEventListener('DOMContentLoaded', function () {
    var want = stored();
    if (!DICT[want]) { want = DEFAULT; }
    if (want !== DEFAULT) { apply(want); } else { apply(DEFAULT); }

    var sels = document.querySelectorAll('.langpick');
    for (var i = 0; i < sels.length; i++) {
      sels[i].addEventListener('change', function (e) {
        var v = e.target.value;
        if (!DICT[v]) { return; }
        apply(v); remember(v);
      });
    }
  });
})();
