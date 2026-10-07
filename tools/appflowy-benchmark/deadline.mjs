// A failure deadline surrounds browser work; it never supplies the reported
// elapsed time. The caller closes the browser if its operation cannot finish.
export async function deadline(promise, milliseconds, context) {
  let timer;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`${context}: deadline exceeded`)), milliseconds);
    })]);
  } finally { clearTimeout(timer); }
}
