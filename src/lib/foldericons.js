// Filled, solid category icons that take a folder's colour. Shared between the
// FolderIcon component and the folder-customization picker.
export const FOLDER_ICONS = {
  folder: '<path d="M3 6a2 2 0 0 1 2-2h4.2a2 2 0 0 1 1.4.6L12.4 6H19a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>',
  star: '<path d="M12 2.5l2.9 5.9 6.5.9-4.7 4.6 1.1 6.5L12 17.8 6.2 20.9l1.1-6.5L2.6 9.3l6.5-.9z"/>',
  bolt: '<path d="M13 2L4.5 13.5H10l-1 8.5L19.5 10H14z"/>',
  doc: '<path d="M6 2h7l5 5v13a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2z"/>',
  chat: '<path d="M5 4h14a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H9.5L5 20v-4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z"/>',
  flag: '<path d="M5 2h1.6v20H5zM7.5 3h10l-2.2 4 2.2 4h-10z"/>',
  heart: '<path d="M12 21S3.5 15.7 3.5 9.4C3.5 6.4 5.8 4.5 8.3 4.5c1.7 0 3 .9 3.7 2 .7-1.1 2-2 3.7-2 2.5 0 4.8 1.9 4.8 4.9C20.5 15.7 12 21 12 21z"/>',
  box: '<path d="M3.5 3.5h7v7h-7zM13.5 3.5h7v7h-7zM3.5 13.5h7v7h-7zM13.5 13.5h7v7h-7z"/>',
  mail: '<path d="M3 5.5A1.5 1.5 0 0 1 4.5 4h15A1.5 1.5 0 0 1 21 5.5L12 12zM3 8l9 6 9-6v10.5A1.5 1.5 0 0 1 19.5 20h-15A1.5 1.5 0 0 1 3 18.5z"/>',
  tag: '<path d="M11.6 2.6 21 12l-8.5 8.5a2 2 0 0 1-2.8 0L2 13V4.4A1.4 1.4 0 0 1 3.4 3h8.2zM7 8a1.4 1.4 0 1 0 0-2.8A1.4 1.4 0 0 0 7 8z"/>',
  sparkle: '<path d="M12 2l2.1 5.7L20 10l-5.9 2.3L12 18l-2.1-5.7L4 10l5.9-2.3z"/>',
  rocket: '<path d="M14 3c4 0 7 3 7 7 0 3-3 6-7 8l-2-2-2 2c-4-2-7-5-7-8 0-4 3-7 7-7zm-1 6a2 2 0 1 0 0 4 2 2 0 0 0 0-4z"/>',
};

export const FOLDER_ICON_NAMES = Object.keys(FOLDER_ICONS);

// A small tasteful palette matching the slate theme; custom colour is offered too.
export const FOLDER_COLORS = [
  "#8b9fa4",
  "#c98b8b",
  "#d0a45f",
  "#cabf6a",
  "#7fb894",
  "#6fa8c9",
  "#8f8bc9",
  "#c98bbd",
  "#aeb6ba",
];
