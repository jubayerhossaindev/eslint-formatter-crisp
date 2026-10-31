/** @type {import('@commitlint/types').UserConfig} */
export default {
  extends: ['@commitlint/config-conventional'],

  rules: {
    'scope-enum': [2, 'always', ['package', 'docs', 'repo']],
    'scope-empty': [2, 'never'],
    'header-max-length': [2, 'always', 100],
    'scope-case': [2, 'always', ['lower-case', 'kebab-case', 'camel-case', 'pascal-case']],
    'body-max-line-length': [0, 'always'],
    'footer-max-line-length': [0, 'always'],
    'type-enum': [
      2,
      'always',
      [
        'build', // বিল্ড সিস্টেম বা এক্সটার্নাল ডিপেনডেন্সি চেঞ্জ
        'chore', // সোর্স বা টেস্ট ফাইল ছাড়া অন্যান্য সাধারণ কাজ
        'ci', // CI/CD কনফিগারেশন সম্পর্কিত চেঞ্জ
        'docs', // শুধু ডকুমেন্টেশন পরিবর্তন
        'feat', // নতুন ফিচার যোগ করা
        'fix', // বাগ ফিক্স করা
        'perf', // পারফরম্যান্স ইমপ্রুভ করার কাজ
        'refactor', // বাগ ফিক্স বা নতুন ফিচার ছাড়া কোড রিফ্যাক্টরিং
        'revert', // আগের কোনো কমিট রোলব্যাক করা
        'style', // কোডের ফরম্যাটিং বা স্টাইল সংক্রান্ত চেঞ্জ
        'test', // টেস্ট ফাইল যোগ বা আপডেট করা
        'deps', // প্যাকেজ/ডিপেনডেন্সি আপডেট
        'wip', // চলমান কাজ (Work in Progress)
      ],
    ],
  },
};
