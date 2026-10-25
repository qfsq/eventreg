const app = getApp();
const config = require('../../config');

const CATEGORIES = [
  { value: 'chore', label: 'Chore' },
  { value: 'work', label: 'Work' },
  { value: 'emotion', label: 'Emotion' },
  { value: 'thinking', label: 'Thinking' },
  { value: 'writing', label: 'Writing' },
  { value: 'inventing', label: 'Inventing' }
];

const LABELS = {};
CATEGORIES.forEach((category) => {
  LABELS[category.value] = category.label;
});

Page({
  data: {
    categoryLabels: CATEGORIES.map((category) => category.label),
    selectedCategoryIndex: 1,
    selectedCategoryLabel: CATEGORIES[1].label,
    title: '',
    content: '',
    summaries: [],
    loading: true,
    submitting: false,
    error: '',
    demoAccount: true
  },

  onShow() {
    this.fetchSummaries();
  },

  onPullDownRefresh() {
    this.fetchSummaries(() => wx.stopPullDownRefresh());
  },

  bindPickerChange(event) {
    const index = Number(event.detail.value);
    this.setData({
      selectedCategoryIndex: index,
      selectedCategoryLabel: CATEGORIES[index].label
    });
  },

  onTitleInput(event) {
    this.setData({ title: event.detail.value });
  },

  onContentInput(event) {
    this.setData({ content: event.detail.value });
  },

  fetchSummaries(done) {
    const userId = app.resolveUserId();
    this.setData({
      loading: true,
      error: '',
      demoAccount: !app.globalData.userId
    });

    wx.request({
      url: `${config.baseUrl}/summaries/${userId}`,
      method: 'GET',
      success: (response) => {
        if (response.statusCode >= 200 && response.statusCode < 300) {
          const summaries = (response.data || []).map((item) =>
            Object.assign({}, item, { label: LABELS[item.category] || item.category })
          );
          this.setData({ summaries: summaries, loading: false });
        } else {
          this.setData({
            loading: false,
            error:
              (response.data && response.data.error) ||
              `Failed to load summaries (HTTP ${response.statusCode})`
          });
        }
      },
      fail: (error) => {
        this.setData({ loading: false, error: error.errMsg || 'Network error' });
      },
      complete: () => {
        if (typeof done === 'function') {
          done();
        }
      }
    });
  },

  submitEvent() {
    if (this.data.submitting) {
      return;
    }

    const title = this.data.title.trim();
    if (!title) {
      wx.showToast({ title: 'Title is required', icon: 'none' });
      return;
    }

    const content = this.data.content.trim();
    this.setData({ submitting: true, error: '' });

    wx.request({
      url: `${config.baseUrl}/events`,
      method: 'POST',
      header: { 'content-type': 'application/json' },
      data: {
        user_id: app.resolveUserId(),
        category: CATEGORIES[this.data.selectedCategoryIndex].value,
        title: title,
        content: content || null
      },
      success: (response) => {
        if (response.statusCode >= 200 && response.statusCode < 300) {
          wx.showToast({ title: 'Logged', icon: 'success' });
          this.setData({ title: '', content: '' });
          this.fetchSummaries();
        } else {
          this.setData({
            error:
              (response.data && response.data.error) ||
              `Failed to log event (HTTP ${response.statusCode})`
          });
        }
      },
      fail: (error) => {
        this.setData({ error: error.errMsg || 'Network error' });
      },
      complete: () => {
        this.setData({ submitting: false });
      }
    });
  }
});
