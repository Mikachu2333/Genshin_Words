import copy
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from curate_dictionary import Curator, Entry, read_dictionary, write_dictionary
from curation_sources import build_sources, field_kind, field_values, name_entries, write_sources
from extract_words import entry_lines, new_text_groups
import merge
from update_dictionary import prepare_batch


def curator(terms=(), furniture=(), split=None, corrections=None, composable=()):
    rules = {
        "max_length": 10, "common_min_frequency": 500,
        "common_words": ["慈爱"], "keep": [], "split": split or {},
        "corrections": corrections or {}, "composable": list(composable),
    }
    lexicon = {word: (1000, "n") for word in ("故事", "教导", "淑女帽", "哲学", "指引", "传统", "地板", "实验室")}
    return Curator(rules, {"terms": {word: ["fixture"] for word in terms},
                           "furniture": {word: ["fixture"] for word in furniture}}, lexicon)


def clean(engine, *words):
    entries, actions = engine.curate([Entry(word) for word in words])
    return {entry.word for entry in entries}, actions


class CurationTests(unittest.TestCase):
    def test_ahmar_only_loses_story(self):
        words = ["阿赫玛尔", "阿赫玛尔的故事", "阿赫玛尔之须", "阿赫玛尔之眼"]
        engine = curator(terms=words, split={words[1]: [words[0], "的", "故事"]})
        result, _ = clean(engine, *words)
        self.assertEqual(result, {words[0], words[2], words[3]})

    def test_talent_books_and_artifact_survive_common_components(self):
        names = ["慈爱的教导", "慈爱的淑女帽", "慈爱的哲学", "慈爱的指引"]
        result, _ = clean(curator(terms=names), "慈爱", *names)
        self.assertEqual(result, set(names))

    def test_protection_wins_over_furniture_and_common_word(self):
        result, _ = clean(curator(terms=["传统地板", "故事"], furniture=["传统地板"]), "传统地板", "故事")
        self.assertEqual(result, {"传统地板", "故事"})

    def test_furniture_can_be_composed(self):
        result, actions = clean(curator(furniture=["传统地板"]), "传统地板")
        self.assertEqual(result, set())
        self.assertEqual(actions[0]["parts"], ["传统", "地板"])

    def test_unknown_proper_noun_is_not_inferred_from_subwords(self):
        engine = curator()
        engine.lexicon.update({"皇家": (10000, "n"), "军事": (10000, "n"), "学院": (10000, "n")})
        self.assertEqual(clean(engine, "皇家军事学院")[0], {"皇家军事学院"})

    def test_single_characters_cannot_explain_a_rare_tail(self):
        engine = curator(composable=["阿赫玛尔之须"])
        result, _ = clean(engine, "阿赫玛尔", "阿赫玛尔之须")
        self.assertIn("阿赫玛尔之须", result)

    def test_long_name_splits_with_complete_proper_noun(self):
        long_name = "幕形态黑体物质笼实验室"
        engine = curator(terms=[long_name, "幕形态黑体物质笼"],
                         split={long_name: ["幕形态黑体物质笼", "实验室"]})
        result, actions = clean(engine, long_name)
        self.assertEqual(result, {"幕形态黑体物质笼"})
        self.assertEqual("".join(actions[0]["parts"]), long_name)
        self.assertTrue(all(len(word) <= 10 for word in result))
        # 重新构造实例，验证持久化规则足够维持结果。
        again = Curator(copy.deepcopy(engine.rules), {"terms": engine.sources}, engine.lexicon)
        self.assertEqual(clean(again, *result)[0], result)

    def test_unknown_long_word_fails_instead_of_cutting_ten_chars(self):
        with self.assertRaisesRegex(ValueError, "无法按词汇"):
            clean(curator(), "甲乙丙丁戊己庚辛壬癸甲")

    def test_automatic_long_split_survives_reload(self):
        engine = curator(terms=["阿赫玛尔", "幕形态黑体物质笼"])
        result, _ = clean(engine, "阿赫玛尔幕形态黑体物质笼")
        again = Curator(copy.deepcopy(engine.rules), {"terms": engine.sources}, engine.lexicon)
        self.assertEqual(clean(again, *result)[0], result)
        self.assertEqual(result, {"阿赫玛尔", "幕形态黑体物质笼"})

    def test_invalid_override_cannot_lose_characters(self):
        with self.assertRaisesRegex(ValueError, "逐字覆盖"):
            curator(split={"阿赫玛尔的故事": ["阿赫玛尔", "故事"]})

    def test_correction_deduplicates_against_existing_correct_word(self):
        engine = curator(corrections={"最胜紫金碎屑": {
            "replacement": "最胜紫晶碎屑", "evidence": ["TextMap:1830354268"]}})
        result, actions = clean(engine, "最胜紫金碎屑", "最胜紫晶碎屑")
        self.assertEqual(result, {"最胜紫晶碎屑"})
        self.assertEqual(actions[0]["action"], "correct")

    def test_distinct_manual_readings_survive(self):
        entries = [Entry("薄缘的道与光与胤", (code, "100")) for code in
                   ("bao yuan de dao yu guang yu yin", "bo yuan de dao yu guang yu yin")]
        output, _ = curator().curate(entries + entries)
        self.assertEqual(output, entries)

    def test_correction_requires_explicit_manual_reading(self):
        engine = curator(corrections={"法珠": {"replacement": "法球", "evidence": ["fixture"]}})
        with self.assertRaisesRegex(ValueError, "手工拼音"):
            engine.curate([Entry("法珠", ("fa zhu", "100"))])

    def test_ten_chars_are_allowed(self):
        word = "甲乙丙丁戊己庚辛壬癸"
        self.assertEqual(clean(curator(terms=[word]), word)[0], {word})

    def test_dictionary_header_and_codes_round_trip(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "test.dict.yaml"
            path.write_text('---\nname: test\n...\n# 注释\n美露莘\tmei lu xin\t100\n', encoding="utf-8")
            header, entries = read_dictionary(path)
            write_dictionary(path, header, entries)
            self.assertEqual(read_dictionary(path), (header, entries))

    def test_invalid_addition_is_rejected(self):
        with self.assertRaises(ValueError):
            Entry("hello")

    def test_shared_furniture_prefix_keeps_different_tail(self):
        names = ["基地的宝库大门", "基地的沉静漫射"]
        engine = curator(furniture=names)
        engine.lexicon["基地"] = (10000, "n")
        engine.families = engine.furniture_families()
        result, actions = clean(engine, *names)
        self.assertEqual(result, {"宝库大门", "沉静漫射"})
        self.assertTrue(all(action["reason"] == "furniture_family" for action in actions))

    def test_new_version_reuses_families_without_old_full_names(self):
        names = ["基地的宝库大门", "基地的沉静漫射", "基地的清凉秘方"]
        engine = curator(furniture=names)
        engine.lexicon["基地"] = (10000, "n")
        engine.families = engine.furniture_families()
        result, _ = clean(engine, "宝库大门", "沉静漫射", names[-1])
        self.assertEqual(result, {"宝库大门", "沉静漫射", "清凉秘方"})

    def test_shared_affix_does_not_split_inside_proper_place(self):
        names = ["正义之廷的分秒", "正义之廷的勇气"]
        engine = curator(furniture=names)
        result, actions = clean(engine, *names)
        self.assertIn("正义之廷", result)
        self.assertEqual(actions[0]["parts"][0], "正义之廷")
        self.assertFalse(any(word.startswith("廷的") for word in result))

    def test_protected_item_wins_over_shared_furniture_family(self):
        names = ["基地的宝库大门", "基地的沉静漫射"]
        engine = curator(terms=names, furniture=names)
        result, _ = clean(engine, *names)
        self.assertEqual(result, set(names))

    def test_many_unknown_long_words_are_reported_together(self):
        names = ["甲乙丙丁戊己庚辛壬癸甲", "乙丙丁戊己庚辛壬癸甲乙"]
        output, actions, pending = curator().review_batch([Entry(word) for word in names] + [Entry("阿赫玛尔")])
        self.assertEqual({item["word"] for item in pending}, set(names))
        self.assertEqual(output, [Entry("阿赫玛尔")])
        self.assertFalse(actions)


class SourceTests(unittest.TestCase):
    def test_name_normalization_keeps_talent_book(self):
        self.assertEqual(name_entries("「慈爱」的教导"), ["慈爱的教导"])
        self.assertEqual(entry_lines("「慈爱」的教导", "name"), ["慈爱的教导"])

    def test_sensitive_descriptions_bypass_destructive_regex(self):
        text = "星霜的流旋造成的伤害提升"
        self.assertEqual(entry_lines(text, "protected_description"), [text])

    def test_all_required_categories_are_protected(self):
        for file, field in [
            ("Achievement.json", "[].Title"), ("Weapon.json", "[].Affix.Name"),
            ("Reliquary.json", "[].Name"), ("Material.json", "[].Name"),
            ("Avatar/1.json", "Name"), ("Avatar/1.json", "FetterInfo.Native"),
            ("Avatar/1.json", "FetterInfo.ConstellationBefore"),
            ("Avatar/1.json", "SkillDepot.Skills[].Name"),
            ("Avatar/1.json", "SkillDepot.Talents[].Name"),
        ]:
            with self.subTest(file=file, field=field):
                self.assertEqual(field_kind(file, field), "name")
        for file, field in [("Achievement.json", "[].Description"),
                            ("Weapon.json", "[].Affix.Descriptions[].Description"),
                            ("Avatar/1.json", "SkillDepot.Skills[].Description")]:
            self.assertEqual(field_kind(file, field), "protected_description")
        self.assertEqual(field_kind("Furniture.json", "[].Name"), "furniture")

    def test_field_location_retained(self):
        self.assertEqual(list(field_values({"Skills": [{"Name": "神里流"}]}, "Skills[].Name")),
                         [("神里流", "$.Skills[0].Name")])

    def test_same_text_newly_becomes_a_name_is_not_lost(self):
        def records(snapshot):
            return [("慈爱的教导", "name" if snapshot == "new" else "prose", "Material.json:$[1].Name")]
        with patch("extract_words.metadata_records", records):
            groups = new_text_groups("old", "new")
        self.assertEqual(groups[0][1][0]["kind"], "name")

    def test_source_serialization_preserves_unicode_and_locations(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sources.json"
            sources = {"metadata_revision": "123", "terms": {"慈爱": ["fixture"]}, "furniture": {}}
            write_sources(path, sources)
            self.assertEqual(json.loads(path.read_text(encoding="utf-8")), sources)

    def test_blueprint_quote_does_not_protect_furniture_again(self):
        records = [
            ("基地的沉静漫射", "furniture", "Furniture.json:$.Name"),
            ("图纸记载着「基地的沉静漫射」的制作方法。", "prose", "Material.json:$.Description"),
            ("完成基地的宝库大门。", "protected_description", "Achievement.json:$.Description"),
            ("基地的宝库大门", "furniture", "Furniture.json:$.Name"),
        ]
        with tempfile.TemporaryDirectory() as directory:
            npc = Path(directory) / "npc.jsonl"
            npc.write_text("", encoding="utf-8")
            with patch("curation_sources.metadata_records", return_value=records):
                sources, _ = build_sources(Path(directory), npc, {"基地的沉静漫射", "基地的宝库大门"}, revision="fixture")
        self.assertNotIn("基地的沉静漫射", sources["terms"])
        self.assertIn("基地的沉静漫射", sources["furniture"])
        self.assertIn("基地的宝库大门", sources["terms"])

    def test_batch_preview_leaves_live_dictionary_unchanged(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            dictionary = root / "test.dict.yaml"
            dictionary.write_text("---\nname: test\n...\n阿赫玛尔\n", encoding="utf-8")
            before = dictionary.read_bytes()
            candidates = root / "candidates.txt"
            candidates.write_text("基地的宝库大门\n基地的沉静漫射\n", encoding="utf-8")
            rules = root / "rules.json"
            rules.write_text(json.dumps(curator().rules, ensure_ascii=False), encoding="utf-8")
            sources = root / "sources.json"
            write_sources(sources, {"terms": {}, "furniture": {
                "基地的宝库大门": ["fixture"], "基地的沉静漫射": ["fixture"]}})
            _, pending = prepare_batch(dictionary, candidates, root / "preview", rules, sources)
            self.assertEqual(dictionary.read_bytes(), before)
            self.assertFalse(pending)
            self.assertEqual(set((root / "preview/additions.txt").read_text(encoding="utf-8").splitlines()),
                             {"宝库大门", "沉静漫射"})


class MergeTests(unittest.TestCase):
    def test_generator_failure_does_not_overwrite_either_dictionary(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            dictionary = base / "yuanshen.dict.yaml"
            fcitx = base / "yuanshen_fcitx5.txt"
            dictionary.write_text("---\nname: test\n...\n阿赫玛尔\n", encoding="utf-8")
            fcitx.write_text("existing", encoding="utf-8")
            before = dictionary.read_bytes(), fcitx.read_bytes()
            with patch("merge.subprocess.run", side_effect=subprocess.CalledProcessError(1, "generator")):
                with self.assertRaises(subprocess.CalledProcessError):
                    merge.merge(dictionary)
            self.assertEqual((dictionary.read_bytes(), fcitx.read_bytes()), before)


if __name__ == "__main__":
    unittest.main()
