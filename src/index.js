function showResults(json, results) {
    const ul = document.createElement('ul')

    for ([path, rank] of json) {
        if (!rank) continue

        console.log(`${path}: ${rank}`)

        const li = document.createElement('li')
        const span_path = document.createElement('span')
        span_path.appendChild(document.createTextNode(path))
        li.appendChild(span_path)

        ul.appendChild(li);
    }

    results.appendChild(ul)
}

async function search(prompt) {
    const response = await fetch("/api/search", {
        method: 'POST',
        headers: {'Content-Type': 'text/plain'},
        body: prompt,
    });

    const json = await response.json();
    return json
}

document.addEventListener("DOMContentLoaded", function(_) {
    const query = document.getElementById("query");
    const results = document.getElementById("results")

    query.addEventListener("keyup", async (e) => {
        const q = e.target.value
        results.innerHTML = ""

        if (!q) return

        const result = await search(q)
        showResults(result, results)
    })
});
