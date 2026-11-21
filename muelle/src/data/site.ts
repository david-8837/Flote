export const site = {
  name: "Flote",
  tagline: "The floating dock, made for Windows.",
  description:
    "Free, open-source macOS-style dock for Windows 10 and 11. Genie animations, window previews, AI usage and optional accessory battery indicators.",
  url: "https://github.com/david-8837/Flote",
  ogImage: "/og-image.png",
  ogImageWidth: 1200,
  ogImageHeight: 630,
  twitterHandle: "",
  locale: "en_US",
  repository: "https://github.com/david-8837/Flote",
  releases: "https://github.com/david-8837/Flote/releases",
  download: "https://github.com/david-8837/Flote/releases/latest/download/Flote-Setup.exe",
  issues: "https://github.com/david-8837/Flote/issues",
  version: "0.1.20",
  license: "MIT",
} as const;

/** The person who makes Muelle. The @id matches the Person node of the crafter.run profile. */
export const author = {
  name: "Cristian Olivera Chávez",
  shortName: "Cristian Olivera",
  role: "Software Engineer",
  location: "Lima, Peru",
  id: "https://crafter.run/en/team/cristianolivera1#person",
  profile: "https://crafter.run/en/team/cristianolivera1",
  github: "https://github.com/CristianOlivera1",
  x: "https://x.com/CrisOliveraDev",
  linkedin: "https://www.linkedin.com/in/cristian-olivera-chavez",
  website: "https://cristianolivera1.github.io",
} as const;

/** The community that publishes Muelle. The @id matches the Organization node of crafter.run. */
export const organization = {
  name: "Crafter Station",
  id: "https://crafter.run/#organization",
  url: "https://crafter.run",
  github: "https://github.com/david-8837",
  products: "https://crafter.run/en/products",
  description: "The open source community of builders in Peru and LATAM.",
} as const;
