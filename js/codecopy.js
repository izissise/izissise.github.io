const changeIcon=(e,t)=>{e.classList.add(t),setTimeout(()=>e.classList.remove(t),2500)},copyCodeAndChangeIcon=async(e,t)=>{const n=t.querySelector("table")?getTableCode(t):getNonTableCode(t);try{await navigator.clipboard.writeText(n),changeIcon(e,"yes")}catch{changeIcon(e,"err")}},getNonTableCode=e=>[...e.querySelectorAll("code")].map(e=>e.textContent).join(""),getTableCode=e=>[...e.querySelectorAll("tr")].map(e=>e.querySelector("td:last-child")?.innerText??"").join("");document.querySelectorAll("pre").forEach(e=>{const t=document.createElement("div");t.className="svg cc copy",t.innerHTML=" ",e.prepend(t),t.addEventListener("click",()=>copyCodeAndChangeIcon(t,e))})