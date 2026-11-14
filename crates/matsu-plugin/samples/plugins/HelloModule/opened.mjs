// The jack DocumentOpen (ES module). What the module keeps in its variables stays between the calls,
// because the module is loaded once and the process stays.
let count = 0;

export function opens() {
  return count;
}

export default function () {
  count += 1;
}
