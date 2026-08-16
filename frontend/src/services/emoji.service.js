import axios from 'axios';

const EMOJIHUB_BASE = 'https://emojihub.yurace.pro/api/all/category';

// EmojiHub's raw `unicode` field is one or more "U+XXXX" code points per
// entry — a multi-codepoint sequence (e.g. a ZWJ combo) arrives as several
// space-separated tokens in one string rather than one token per array
// element, so this flattens both shapes the same way before decoding.
function unicodeToChar(unicodeEntries) {
  return unicodeEntries
    .flatMap((entry) => entry.split(' '))
    .map((codePoint) => String.fromCodePoint(parseInt(codePoint.replace('U+', ''), 16)))
    .join('');
}

// A plain axios call, not the app's shared `api` instance (src/lib/axios.js):
// EmojiHub is a public third-party API with no relationship to our backend —
// it needs neither the access-token interceptor nor the refresh-on-401 retry
// logic that instance carries.
export function fetchEmojiCategory(category) {
  return axios.get(`${EMOJIHUB_BASE}/${category}`).then((res) =>
    res.data.map((entry) => ({
      name: entry.name,
      char: unicodeToChar(entry.unicode),
    })),
  );
}
