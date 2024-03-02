import json
from collections import defaultdict
import string
import os
from yaml import load, dump
import csv
from difflib import SequenceMatcher

try:
    from yaml import CLoader as Loader, CDumper as Dumper
except ImportError:
    from yaml import Loader, Dumper


def split_by_continent():
    with open("country_continent.json") as f:
        data = json.load(f)
    res = defaultdict(list)
    for obj in data:
        res[obj["continent"]].append(obj["country"])
    for v in res.values():
        v.sort()
    with open("continent_to_country.json", "w") as f:
        json.dump(res, f, indent=2)


def continent_fixup():
    with open("country_continent.json") as f:
        countries = json.load(f)
    for c in countries:
        if c["country"] == "Holy See (Vatican City State)":
            c["country"] = "Vatican City"
    countries.sort(key=lambda x: x["country"])
    with open("country_continent.json", "w") as f:
        json.dump(countries, f, indent=2)


def capital_fixup():
    with open("country_capital.json") as f:
        capitals = json.load(f)
    capitals = {c["country"]: c["city"] for c in capitals}

    capitals["Belgium"] = "Brussels"
    capitals["Colombia"] = "Bogota"
    capitals["Finland"] = "Helsinki"
    capitals.pop("Holy See (Vatican City State)", None)
    capitals["Vatican City"] = "Vatican City"
    capitals["Luxembourg"] = "Luxembourg"
    capitals["Marshall Islands"] = "Majuro"
    capitals["Mexico"] = "Mexico City"
    capitals["Myanmar"] = "Naypyidaw"
    capitals["Palau"] = "Ngerulmud"
    capitals["Palestine"] = "Ramallah"
    capitals["Panama"] = "Panama City"
    capitals["Sri Lanka"] = "Colombo"
    capitals["Togo"] = "Lome"
    capitals["Western Sahara"] = "Laayoune"
    capitals["Chile"] = "Santiago"
    capitals["Cuba"] = "Havana"
    capitals["Dominican Republic"] = "Santo Domingo"
    capitals["Guatemala"] = "Guatemala City"
    capitals["Cook Islands"] = "Avarua"
    capitals["China"] = "Beijing"
    capitals["Bahrain"] = "Manama"
    capitals["Mongolia"] = "Ulaanbaatar"
    capitals["Oman"] = "Muscat"
    capitals["Uzbekistan"] = "Tashkent"
    capitals["Austria"] = "Vienna"
    capitals["Czech Republic"] = "Prague"
    capitals["Faroe Islands"] = "Torshavn"
    capitals["Greece"] = "Athens"
    capitals["Monaco"] = "Monaco"
    capitals["Italy"] = "Rome"
    capitals["Romania"] = "Bucharest"
    capitals["Portugal"] = "Lisbon"
    capitals["Poland"] = "Warsaw"

    capitals = [{"country": k, "city": v} for k, v in capitals.items()]
    capitals.sort(key=lambda x: x["country"])
    with open("country_capital.json", "w") as f:
        json.dump(capitals, f, indent=2)


def continent_to_capitals():
    with open("continent_to_country.json") as f:
        continents = json.load(f)
    with open("country_capital.json") as f:
        capitals = json.load(f)
    printable = set(string.printable)
    fprint = lambda s: "".join(ss for ss in s if ss in printable)
    for cont, countries in continents.items():
        cs = [cc for cc in capitals if cc["country"] in countries and cc["city"]]
        cs = [{fprint(k): fprint(v) for k, v in c.items()} for c in cs]
        if len(cs) == 0:
            continue
        cont = cont.replace(" ", "_").lower()
        with open(f"{cont}_capitals.json", "w") as f:
            json.dump(cs, f, indent=2)


def continent_capitals_questions():
    for p in os.listdir("."):
        if not p.endswith("_capitals.json"):
            continue
        with open(p) as f:
            data = json.load(f)
        continent = p.removesuffix("_capitals.json")
        items = [
            {
                "question": d["country"],
                "answers": [d["city"]],
            }
            for d in data
        ]
        items.sort(key=lambda x: x["question"])
        questions = {
            "name": continent + "_capitals",
            "type_": "default",
            "data": {
                "question_prefix": "What is the capital of ",
                "items": items,
            },
        }
        with open(f"questions/{continent}_capitals.yaml", "w") as f:
            dump(questions, f, Dumper=Dumper)


def continent_to_areas():
    with open("continent_to_country.json") as f:
        continents = json.load(f)
    with open("country_area.json") as f:
        areas = json.load(f)
    min_area = 1000
    areas = [item for item in areas if item["area"] > min_area]
    # printable = set(string.printable)
    # fprint = lambda s: "".join(ss for ss in s if ss in printable)
    for continent, countries in continents.items():
        continent_areas = [item for item in areas if item["country"] in countries]
        if len(continent_areas) == 0:
            continue
        continent = continent.replace(" ", "_").lower()
        with open(f"{continent}_areas.json", "w") as f:
            json.dump(continent_areas, f, indent=2)


def continent_area_questions():
    for p in os.listdir("."):
        if not p.endswith("_areas.json"):
            continue
        with open(p) as f:
            data = json.load(f)
        continent = p.removesuffix("_areas.json")
        items = [
            {
                "question": d["country"],
                "answer": int(d["area"]),
            }
            for d in data
        ]
        items.sort(key=lambda x: x["question"])
        questions = {
            "name": continent + "_areas",
            "type_": "numeric_range",
            "data": {
                "question_prefix": "What is the area (km^2) of ",
                "items": items,
                "range": 0.025,
            },
        }
        with open(f"questions/{continent}_areas.yaml", "w") as f:
            dump(questions, f, Dumper=Dumper)


def gen_capital_questions():
    continent_fixup()
    split_by_continent()
    capital_fixup()
    continent_to_capitals()
    continent_capitals_questions()


def gen_area_questions():
    continent_fixup()
    split_by_continent()
    continent_to_areas()
    continent_area_questions()


def population_json():
    populations = {}
    with open("world_population.csv") as f:
        reader = csv.reader(f)
        for line in reader:
            country, pop = line[3], line[4]
            population = int(pop.replace(",", "")) * 1000
            populations[country.strip()] = population

    renames = [
        ("Egypt, Arab Rep.", "Egypt"),
        ("Congo, Dem. Rep.", "The Democratic Repuplic of Congo"),
        ("Viet Nam", "Vietnam"),
        ("Iran, Islamic Rep.", "Iran"),
        ("Türkiye", "Turkey"),
        ("Korea, Rep.", "South Korea"),
        ("Yemen, Rep.", "Yemen"),
        ("Venezuela, RB", "Venezuela"),
        ("Côte d'Ivoire", "Ivory Coast"),
        ("Korea, Dem. People's Rep.", "North Korea"),
        ("Syrian Arab Republic", "Syria"),
        ("Lao PDR", "Laos"),
        ("Hong Kong SAR, China", "Hong Kong"),
        ("Kyrgyz Republic", "Kyrgyzstan"),
        ("Libya", "Libyan Arab Jamahiriya"),
        ("Congo, Rep.", "Congo"),
        ("Slovak Republic", "Slovakia"),
        ("West Bank and Gaza", "Palestine"),
        ("Gambia, The", "Gambia"),
        ("Timor-Leste", "East Timor"),
        ("Eswatini", "Swaziland"),
        ("Fiji", "Fiji Islands"),
        ("Macao SAR, China", "Macao"),
        ("Cabo Verde", "Cape Verde"),
        ("Brunei Darussalam", "Brunei"),
        ("Bahamas, The", "Bahamas"),
        ("São Tomé and Principe", "Sao Tome and Principe"),
        ("St. Lucia", "Saint Lucia"),
        ("Curaçao", "Netherlands Antilles"),
        ("Micronesia, Fed. Sts.", "Micronesia, Federated States of"),
        ("Virgin Islands (U.S.)", "Virgin Islands, U.S."),
        ("St. Vincent and the Grenadines", "Saint Vincent and the Grenadines"),
        ("St. Kitts and Nevis", "Saint Kitts and Nevis"),
        ("British Virgin Islands", "Virgin Islands, British"),
    ]
    for a, b in renames:
        rename(populations, a, b)
    populations = [{"country": k, "population": v} for k, v in populations.items()]
    with open("country_population.json", "w") as f:
        json.dump(populations, f, indent=2)


def rename(d: dict, key1: str, key2: str):
    obj = d[key1]
    del d[key1]
    d[key2] = obj


def continent_to_population():
    with open("continent_to_country.json") as f:
        continents = json.load(f)
    with open("country_population.json") as f:
        populations = json.load(f)

    for continent, countries in continents.items():
        continent = continent.replace(" ", "_").lower()
        continent_pops = [c for c in populations if c["country"] in countries]
        with open(f"{continent}_populations.json", "w") as f:
            json.dump(continent_pops, f, indent=2)


def continent_population_questions():
    for p in os.listdir("."):
        if not p.endswith("_populations.json"):
            continue
        with open(p) as f:
            data = json.load(f)
        continent = p.removesuffix("_populations.json")
        items = [
            {
                "question": d["country"],
                "answer": int(d["population"]),
            }
            for d in data
        ]
        items.sort(key=lambda x: x["question"])
        questions = {
            "name": continent + "_populations",
            "type_": "numeric_range",
            "data": {
                "question_prefix": "What is the population of ",
                "items": items,
                "range": 0.025,
            },
        }
        with open(f"questions/{continent}_populations.yaml", "w") as f:
            dump(questions, f, Dumper=Dumper)


def gen_population_questions():
    continent_fixup()
    split_by_continent()
    population_json()
    continent_to_population()
    continent_population_questions()


gen_capital_questions()
gen_area_questions()
gen_population_questions()
