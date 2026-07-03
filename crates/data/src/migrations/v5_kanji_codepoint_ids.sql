-- v5: remap user-state kanji ids from seed auto-rowids to Unicode codepoints.
--
-- Seeds up to v0.3 inserted kanji without an explicit id, so kanji.id was the SQLite
-- auto-rowid = insertion order (levels in learning order, glyphs codepoint-sorted within
-- level). That id renumbers whenever JLPT level membership shifts (e.g. when N3 lands),
-- which would silently reattach all user progress to the wrong kanji. From v0.4 the seed
-- uses the glyph's Unicode codepoint as kanji.id (stable forever); this migration remaps
-- existing user data with the frozen historical rowid->glyph->codepoint mapping below.
--
-- Mapping provenance: generated 2026-07-03 from the last pre-change seed
-- (`SELECT id, glyph FROM kanji ORDER BY id`); its 79-row N5 prefix was verified
-- byte-identical to the seed shipped in v0.1.x, and v0.2/v0.3 shipped all 245 rows in this
-- exact order. FROZEN HISTORICAL DATA - never regenerate, even when levels reshuffle.
--
-- Safety properties:
--   * Old ids (1..245) and codepoints (>= U+3400) are disjoint and the map is injective,
--     so remapping a DB written by any shipped build cannot collide.
--   * Mixed old/new ids in one DB (dev artifact): the UPDATE trips the track/user_mnemonic
--     PK, the whole migration transaction rolls back, and the DB is left untouched at v4
--     (fail closed - no merge heuristics). review_event has no such uniqueness, so for a
--     mixed DB whose *tracks* do not collide, old-id events merge into the codepoint's
--     history: accepted, since events are only consumed as date/count aggregates and any
--     dangerous mixed-track DB fails closed above. Impossible for shipped DBs either way.
--   * Ids in neither set are left untouched and counted in state_meta (v5_*_invalid);
--     the engine quarantines such orphans from scheduling, budgets, and stats.

-- Durable audit trail of what migrations did (first used by v5 below).
CREATE TABLE state_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE _kanji_id_remap (
    old_id INTEGER PRIMARY KEY,
    glyph  TEXT NOT NULL UNIQUE,
    new_id INTEGER NOT NULL UNIQUE
);

INSERT INTO _kanji_id_remap (old_id, glyph, new_id) VALUES
(1,'一',19968),
(2,'七',19971),
(3,'万',19975),
(4,'三',19977),
(5,'上',19978),
(6,'下',19979),
(7,'中',20013),
(8,'九',20061),
(9,'二',20108),
(10,'五',20116),
(11,'人',20154),
(12,'今',20170),
(13,'休',20241),
(14,'何',20309),
(15,'先',20808),
(16,'入',20837),
(17,'八',20843),
(18,'六',20845),
(19,'円',20870),
(20,'出',20986),
(21,'前',21069),
(22,'北',21271),
(23,'十',21313),
(24,'千',21315),
(25,'午',21320),
(26,'半',21322),
(27,'南',21335),
(28,'友',21451),
(29,'右',21491),
(30,'名',21517),
(31,'四',22235),
(32,'国',22269),
(33,'土',22303),
(34,'外',22806),
(35,'大',22823),
(36,'天',22825),
(37,'女',22899),
(38,'子',23376),
(39,'学',23398),
(40,'小',23567),
(41,'山',23665),
(42,'川',24029),
(43,'左',24038),
(44,'年',24180),
(45,'後',24460),
(46,'日',26085),
(47,'時',26178),
(48,'書',26360),
(49,'月',26376),
(50,'木',26408),
(51,'本',26412),
(52,'来',26469),
(53,'東',26481),
(54,'校',26657),
(55,'母',27597),
(56,'毎',27598),
(57,'気',27671),
(58,'水',27700),
(59,'火',28779),
(60,'父',29238),
(61,'生',29983),
(62,'男',30007),
(63,'白',30333),
(64,'百',30334),
(65,'聞',32862),
(66,'行',34892),
(67,'西',35199),
(68,'見',35211),
(69,'話',35441),
(70,'語',35486),
(71,'読',35501),
(72,'車',36554),
(73,'金',37329),
(74,'長',38263),
(75,'間',38291),
(76,'雨',38632),
(77,'電',38651),
(78,'食',39135),
(79,'高',39640),
(80,'不',19981),
(81,'世',19990),
(82,'主',20027),
(83,'事',20107),
(84,'京',20140),
(85,'仕',20181),
(86,'代',20195),
(87,'以',20197),
(88,'会',20250),
(89,'住',20303),
(90,'体',20307),
(91,'作',20316),
(92,'使',20351),
(93,'借',20511),
(94,'元',20803),
(95,'兄',20804),
(96,'公',20844),
(97,'写',20889),
(98,'冬',20908),
(99,'切',20999),
(100,'別',21029),
(101,'力',21147),
(102,'勉',21193),
(103,'動',21205),
(104,'医',21307),
(105,'去',21435),
(106,'口',21475),
(107,'古',21476),
(108,'台',21488),
(109,'同',21516),
(110,'味',21619),
(111,'品',21697),
(112,'員',21729),
(113,'問',21839),
(114,'図',22259),
(115,'地',22320),
(116,'堂',22530),
(117,'場',22580),
(118,'売',22770),
(119,'夏',22799),
(120,'夕',22805),
(121,'多',22810),
(122,'夜',22812),
(123,'妹',22969),
(124,'姉',22985),
(125,'始',22987),
(126,'字',23383),
(127,'安',23433),
(128,'室',23460),
(129,'家',23478),
(130,'少',23569),
(131,'屋',23627),
(132,'工',24037),
(133,'帰',24112),
(134,'広',24195),
(135,'店',24215),
(136,'度',24230),
(137,'建',24314),
(138,'弟',24351),
(139,'強',24375),
(140,'待',24453),
(141,'心',24515),
(142,'思',24605),
(143,'急',24613),
(144,'悪',24746),
(145,'意',24847),
(146,'手',25163),
(147,'持',25345),
(148,'教',25945),
(149,'文',25991),
(150,'料',26009),
(151,'新',26032),
(152,'方',26041),
(153,'旅',26053),
(154,'族',26063),
(155,'早',26089),
(156,'明',26126),
(157,'映',26144),
(158,'春',26149),
(159,'昼',26172),
(160,'曜',26332),
(161,'有',26377),
(162,'服',26381),
(163,'朝',26397),
(164,'業',26989),
(165,'楽',27005),
(166,'歌',27468),
(167,'止',27490),
(168,'正',27491),
(169,'歩',27497),
(170,'死',27515),
(171,'注',27880),
(172,'洋',27915),
(173,'海',28023),
(174,'漢',28450),
(175,'牛',29275),
(176,'物',29289),
(177,'特',29305),
(178,'犬',29356),
(179,'理',29702),
(180,'用',29992),
(181,'田',30000),
(182,'町',30010),
(183,'画',30011),
(184,'界',30028),
(185,'病',30149),
(186,'発',30330),
(187,'目',30446),
(188,'真',30495),
(189,'着',30528),
(190,'知',30693),
(191,'研',30740),
(192,'社',31038),
(193,'私',31169),
(194,'秋',31179),
(195,'究',31350),
(196,'空',31354),
(197,'立',31435),
(198,'答',31572),
(199,'紙',32025),
(200,'終',32066),
(201,'習',32722),
(202,'考',32771),
(203,'者',32773),
(204,'肉',32905),
(205,'自',33258),
(206,'色',33394),
(207,'花',33457),
(208,'英',33521),
(209,'茶',33590),
(210,'親',35242),
(211,'言',35328),
(212,'計',35336),
(213,'試',35430),
(214,'買',36023),
(215,'貸',36024),
(216,'質',36074),
(217,'赤',36196),
(218,'走',36208),
(219,'起',36215),
(220,'足',36275),
(221,'転',36578),
(222,'近',36817),
(223,'送',36865),
(224,'通',36890),
(225,'週',36913),
(226,'運',36939),
(227,'道',36947),
(228,'重',37325),
(229,'野',37326),
(230,'銀',37504),
(231,'開',38283),
(232,'院',38498),
(233,'集',38598),
(234,'青',38738),
(235,'音',38899),
(236,'題',38988),
(237,'風',39080),
(238,'飯',39151),
(239,'飲',39154),
(240,'館',39208),
(241,'駅',39365),
(242,'験',39443),
(243,'魚',39770),
(244,'鳥',40165),
(245,'黒',40658);

-- Audit (pre-remap): ids that are neither a shipped old rowid nor a known codepoint.
-- The remap below only moves old_id -> new_id, so these counts are stable across it.
INSERT INTO state_meta (key, value)
SELECT 'v5_track_invalid', COUNT(*) FROM track
 WHERE kanji_id NOT IN (SELECT old_id FROM _kanji_id_remap)
   AND kanji_id NOT IN (SELECT new_id FROM _kanji_id_remap);
INSERT INTO state_meta (key, value)
SELECT 'v5_review_event_invalid', COUNT(*) FROM review_event
 WHERE kanji_id NOT IN (SELECT old_id FROM _kanji_id_remap)
   AND kanji_id NOT IN (SELECT new_id FROM _kanji_id_remap);
INSERT INTO state_meta (key, value)
SELECT 'v5_user_mnemonic_invalid', COUNT(*) FROM user_mnemonic
 WHERE kanji_id NOT IN (SELECT old_id FROM _kanji_id_remap)
   AND kanji_id NOT IN (SELECT new_id FROM _kanji_id_remap);

-- The remap. Each changes() capture must immediately follow its UPDATE.
UPDATE track
   SET kanji_id = (SELECT new_id FROM _kanji_id_remap m WHERE m.old_id = track.kanji_id)
 WHERE kanji_id IN (SELECT old_id FROM _kanji_id_remap);
INSERT INTO state_meta (key, value) SELECT 'v5_track_remapped', changes();

UPDATE review_event
   SET kanji_id = (SELECT new_id FROM _kanji_id_remap m WHERE m.old_id = review_event.kanji_id)
 WHERE kanji_id IN (SELECT old_id FROM _kanji_id_remap);
INSERT INTO state_meta (key, value) SELECT 'v5_review_event_remapped', changes();

UPDATE user_mnemonic
   SET kanji_id = (SELECT new_id FROM _kanji_id_remap m WHERE m.old_id = user_mnemonic.kanji_id)
 WHERE kanji_id IN (SELECT old_id FROM _kanji_id_remap);
INSERT INTO state_meta (key, value) SELECT 'v5_user_mnemonic_remapped', changes();

DROP TABLE _kanji_id_remap;
