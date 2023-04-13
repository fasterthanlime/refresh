import { Handlers, PageProps } from "$fresh/server.ts";

interface Data {
  results: Result[];
}

// Sample JSON payload:
/*
[
  {
    "word": "hello",
    "phonetics": [
      {
        "audio": "https://api.dictionaryapi.dev/media/pronunciations/en/hello-au.mp3",
        "sourceUrl": "https://commons.wikimedia.org/w/index.php?curid=75797336",
        "license": {
          "name": "BY-SA 4.0",
          "url": "https://creativecommons.org/licenses/by-sa/4.0"
        }
      },
      {
        "text": "/həˈləʊ/",
        "audio": "https://api.dictionaryapi.dev/media/pronunciations/en/hello-uk.mp3",
        "sourceUrl": "https://commons.wikimedia.org/w/index.php?curid=9021983",
        "license": {
          "name": "BY 3.0 US",
          "url": "https://creativecommons.org/licenses/by/3.0/us"
        }
      },
      { "text": "/həˈloʊ/", "audio": "" }
    ],
    "meanings": [
      {
        "partOfSpeech": "noun",
        "definitions": [
          {
            "definition": "\"Hello!\" or an equivalent greeting.",
            "synonyms": [],
            "antonyms": []
          }
        ],
        "synonyms": ["greeting"],
        "antonyms": []
      },
      {
        "partOfSpeech": "verb",
        "definitions": [
          {
            "definition": "To greet with \"hello\".",
            "synonyms": [],
            "antonyms": []
          }
        ],
        "synonyms": [],
        "antonyms": []
      },
      {
        "partOfSpeech": "interjection",
        "definitions": [
          {
            "definition": "A greeting (salutation) said when meeting someone or acknowledging someone’s arrival or presence.",
            "synonyms": [],
            "antonyms": [],
            "example": "Hello, everyone."
          },
          {
            "definition": "A greeting used when answering the telephone.",
            "synonyms": [],
            "antonyms": [],
            "example": "Hello? How may I help you?"
          },
          {
            "definition": "A call for response if it is not clear if anyone is present or listening, or if a telephone conversation may have been disconnected.",
            "synonyms": [],
            "antonyms": [],
            "example": "Hello? Is anyone there?"
          },
          {
            "definition": "Used sarcastically to imply that the person addressed or referred to has done something the speaker or writer considers to be foolish.",
            "synonyms": [],
            "antonyms": [],
            "example": "You just tried to start your car with your cell phone. Hello?"
          },
          {
            "definition": "An expression of puzzlement or discovery.",
            "synonyms": [],
            "antonyms": [],
            "example": "Hello! What’s going on here?"
          }
        ],
        "synonyms": [],
        "antonyms": ["bye", "goodbye"]
      }
    ],
    "license": {
      "name": "CC BY-SA 3.0",
      "url": "https://creativecommons.org/licenses/by-sa/3.0"
    },
    "sourceUrls": ["https://en.wiktionary.org/wiki/hello"]
  }
]
*/
interface Result {
  word: string;
  phonetics: Phonetics[];
  meanings: Meaning[];
}

interface Phonetics {
  audio: string;
  sourceUrl: string;
}

interface Meaning {
  partOfSpeech: string;
  definitions: Definition[];
  synonyms: string[];
  antonyms: string[];
}

interface Definition {
  definition: string;
  synonyms: string[];
  antonyms: string[];
  example: string;
}

export const handler: Handlers<Data> = {
  async GET(req, ctx) {
    const url = new URL(req.url);
    const query = url.searchParams.get("q") || "";
    const resp = await fetch(
      `https://api.dictionaryapi.dev/api/v2/entries/en/${query}`,
    );
    const results = (await resp.json()) as Result[];
    return ctx.render({ results });
  },
};

export default function Greet(props: PageProps) {
  let results = (props.data as Data).results;

  return (
    <div>
      {results.map((result) => (
        <div class="m-8">
          <h1 class="text-4xl my-4">{result.word}</h1>
          <h2 class="text-2xl">Phonetics</h2>
          <ul class="m-8">
            {result.phonetics.map((phonetic) => (
              <li class="my-4">
                <audio controls src={phonetic.audio} class="inline mr-4" />
                <a href={phonetic.sourceUrl}>Source</a>
              </li>
            ))}
          </ul>
          <h2 class="text-2xl">Meanings</h2>
          <ul class="m-8">
            {result.meanings.map((meaning) => (
              <li>
                <h3>{meaning.partOfSpeech}</h3>
                <ul class="list-disc m-8">
                  {meaning.definitions.map((definition) => (
                    <li>
                      <p>{definition.definition}</p>
                      {definition.example && (
                        <p class="font-bold">Example: {definition.example}</p>
                      )}
                      {definition.antonyms.length > 0 && (
                        <p>Antonyms: {definition.antonyms.join(", ")}</p>
                      )}
                      {definition.synonyms.length > 0 && (
                        <p>Synonyms: {definition.synonyms.join(", ")}</p>
                      )}
                    </li>
                  ))}
                </ul>
                {meaning.antonyms.length > 0 && (
                  <p>Antonyms: {meaning.antonyms.join(", ")}</p>
                )}
                {meaning.synonyms.length > 0 && (
                  <p>Synonyms: {meaning.synonyms.join(", ")}</p>
                )}
              </li>
            ))}
          </ul>
        </div>
      ))}
    </div>
  );
}
