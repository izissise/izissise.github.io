function setTheme(mode) {
    localStorage.setItem("theme-storage", mode);
    let htmlElement = document.querySelector("html");

    if (mode === "dark") {
        document.getElementById("darkModeStyle").disabled = false;
        htmlElement.classList.remove("light")
        htmlElement.classList.add("dark")

        document.getElementById("dark-mode-toggle").children[0].style = "";
    } else if (mode === "light") {
        document.getElementById("darkModeStyle").disabled = true;
        htmlElement.classList.remove("dark")
        htmlElement.classList.add("light")

        document.getElementById("dark-mode-toggle").children[0].style = "filter: invert(1);";
    }
}

function toggleTheme() {
    if (localStorage.getItem("theme-storage") === "light") {
        setTheme("dark");
    } else if (localStorage.getItem("theme-storage") === "dark") {
        setTheme("light");
    }
}

function getSavedTheme() {
    let currentTheme = localStorage.getItem("theme-storage");
    if(!currentTheme) {
        if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
            currentTheme = "dark";
        } else {
            currentTheme = "light";
        }
    }

    return currentTheme;
}

setTheme(getSavedTheme());
